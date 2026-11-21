use crate::database::ids::id_to_sql;
use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::auth::email_in_use::EmailInUseQueryView;
use crate::database::auth::user_password::{GetUserPasswordQueryView, UserPassword};
use crate::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use crate::database::users::patch_user::PatchUserQueryView;
use crate::email_change;
use crate::emails::AccountEmails;
use crate::endpoints::v1::auth::login::endpoint::is_password_valid;
use crate::endpoints::v1::user::me::patch::view::PatchMeView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{too_many_requests_response, user_key, RateLimits};

#[derive(Debug, Clone, PartialEq)]
enum PatchMeError {
    BadRequest(String),
    EmailAlreadyUsed,
    DatabaseError,
    /// The confirmation e-mail could not be sent (or no mailer is configured): nothing was changed.
    Mail,
    /// Redis refused to record the pending change.
    Redis,
    WrongPassword,
    /// The user used up their budget of password checks (MAIR-746): seconds before retrying.
    TooManyRequests(u64),
}

impl std::fmt::Display for PatchMeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message) => write!(f, "{message}"),
            Self::EmailAlreadyUsed => {
                write!(f, "Another account already uses this e-mail address.")
            }
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Mail => write!(f, "An error occurred while sending the email."),
            Self::Redis => write!(f, "An error occurred while accessing Redis."),
            Self::WrongPassword => write!(f, "The current password is incorrect."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
        }
    }
}

impl ResponseError for PatchMeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::EmailAlreadyUsed => StatusCode::CONFLICT,
            Self::DatabaseError | Self::Mail | Self::Redis => StatusCode::INTERNAL_SERVER_ERROR,
            Self::WrongPassword => StatusCode::FORBIDDEN,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::TooManyRequests(retry_after) => too_many_requests_response(*retry_after),
            _ => HttpResponse::build(self.status_code()).body(self.to_string()),
        }
    }
}

/// Changing the e-mail address requires the current password (MAIR-390). Accounts without a
/// password cannot change it here.
async fn check_current_password(
    state: &AppState,
    user_id: u64,
    password: &str,
) -> Result<(), PatchMeError> {
    let stored: UserPassword = state
        .get_smart_db()
        .fetch_one(&GetUserPasswordQueryView::new(user_id))
        .await
        .map_err(|e| {
            tracing::error!("Patch me DB Error: {e}");
            PatchMeError::DatabaseError
        })?;
    match stored.password() {
        Some(hash) if is_password_valid(id_to_sql(user_id), password, hash).await => Ok(()),
        _ => Err(PatchMeError::WrongPassword),
    }
}

/// Starts the change of the e-mail address: the new address is not written, it gets a
/// confirmation link and the account keeps its current address until the link is used. An address
/// already held by an account is refused without saying whose it is.
async fn request_email_change(
    state: &AppState,
    emails: Option<&AccountEmails>,
    user_id: u64,
    new_email: &str,
) -> Result<(), PatchMeError> {
    let smart_db = state.get_smart_db();
    let current: GetUserByIdQueryResultView = smart_db
        .fetch_one(&GetUserByIdQueryView::new(user_id))
        .await
        .map_err(|e| {
            tracing::error!("Patch me DB Error: {e}");
            PatchMeError::DatabaseError
        })?;
    let redis = state.get_redis();
    if new_email.eq_ignore_ascii_case(current.email()) {
        // Back to the current address: a request still waiting is no longer wanted.
        return email_change::cancel(redis, user_id).await.map_err(|e| {
            tracing::error!("Patch me Redis Error: {e}");
            PatchMeError::Redis
        });
    }
    let in_use: i32 = smart_db
        .fetch_scalar(&EmailInUseQueryView::new(new_email))
        .await
        .map_err(|e| {
            tracing::error!("Patch me DB Error: {e}");
            PatchMeError::DatabaseError
        })?;
    if in_use != 0 {
        return Err(PatchMeError::EmailAlreadyUsed);
    }
    let Some(emails) = emails else {
        tracing::error!("Patch me: no mailer is registered, the confirmation cannot be sent");
        return Err(PatchMeError::Mail);
    };
    let token = email_change::new_token();
    if let Err(e) = emails
        .send_email_change_confirmation(new_email, current.first_name(), &token)
        .await
    {
        // The request already waiting, if any, is left as it was: its link was delivered.
        tracing::error!("Patch me: the confirmation e-mail could not be sent: {e}");
        return Err(PatchMeError::Mail);
    }
    email_change::publish(redis, user_id, new_email, &token)
        .await
        .map_err(|e| {
            tracing::error!("Patch me Redis Error: {e}");
            PatchMeError::Redis
        })?;
    Ok(())
}

async fn trigger_patch_me(
    state: web::Data<AppState>,
    emails: Option<&AccountEmails>,
    view: PatchMeView,
    user_id: u64,
    limits: Option<&RateLimits>,
) -> Result<(), PatchMeError> {
    if view.email().is_some() {
        // MAIR-746: every check spends the user's budget, right or wrong, and before the hash is
        // compared, so parallel guesses cannot all slip in before the first failure is counted.
        if let Some(limits) = limits {
            limits
                .password_check_per_user
                .hit(&user_key(user_id))
                .map_err(|refused| {
                    tracing::warn!(
                        user_id,
                        retry_after = refused.retry_after_seconds,
                        "Current password checks rate limited on PATCH /user/me"
                    );
                    PatchMeError::TooManyRequests(refused.retry_after_seconds)
                })?;
        }
        check_current_password(&state, user_id, view.current_password().unwrap_or_default())
            .await?;
    }
    let phone = view
        .phone_change()
        .map_err(|e| PatchMeError::BadRequest(e.to_string()))?;
    // Before anything is written, so a refused or unsent confirmation changes nothing.
    if let Some(new_email) = view.email() {
        request_email_change(&state, emails, user_id, new_email).await?;
    }
    let db_view = PatchUserQueryView::new(
        user_id,
        view.first_name(),
        view.last_name(),
        None,
        &phone,
        None,
    );
    if !db_view.is_noop() {
        state
            .get_smart_db()
            .execute(db_view)
            .await
            .map_err(|e| match e {
                ApiLibError::Database(DbError::UniqueViolation(_)) => {
                    PatchMeError::EmailAlreadyUsed
                }
                e => {
                    tracing::error!("Error: {e:?}");
                    PatchMeError::DatabaseError
                }
            })?;
    }
    Ok(())
}

#[utoipa::path(
    patch,
    path = "/",
    summary = "Update one's own profile",
    description = "Updates the identity, the e-mail address or the phone of the user \
                   of the JWT. Partial update: only the fields present in the \
                   body are written, the others stay unchanged.\n\n\
                   An empty body is accepted and writes nothing: the response stays \
                   `200`. A `null` field is ignored, except `phone`: `null` or `\"\"` removes \
                   the phone (MAIR-480).\n\n\
                   The phone is sent as typed, in national format with \
                   `phone_country` (`\"06 12 34 56 78\"` + `\"FR\"`) or in E.164 \
                   (`\"+33612345678\"`). It is validated against the numbering plan of the country and \
                   read back in E.164 by `GET /api/v1/user/me/`.\n\n\
                   **E-mail address.** A new `email` (with `current_password`) is not written: a \
                   confirmation link, valid one hour, is e-mailed to the NEW address and the account keeps \
                   its current address until `POST /api/v1/auth/confirm_email` is called with that \
                   link's token. The address waiting for confirmation is read as `pending_email` by \
                   `GET /api/v1/user/me/`; a newer request replaces it, and sending the current \
                   address again cancels it. The other fields are written at once.\n\n\
                   The password and the roles are not changed here: use \
                   `/api/v1/auth/forgot_password` for the password and \
                   `/api/v1/admin/users/` for the roles. The response has an empty body; call \
                   `GET /api/v1/user/me/` again to read the profile back.",
    request_body(
        content = PatchMeView,
        description = "Fields to change, all optional: absent or `null` is ignored, except `phone` where `null` or `\"\"` removes the phone.",
        examples(
            ("Change the phone" = (value = json!({ "first_name": "Jean", "phone": "06 12 34 56 78", "phone_country": "FR" }))),
            ("International number" = (value = json!({ "phone": "+32 470 12 34 56" }))),
            ("Remove the phone" = (value = json!({ "phone": null })))
        )
    ),
    responses(
        (
            status = 200,
            description = "Profile updated, or nothing to update; a new `email` is pending confirmation (see `pending_email` of `GET /api/v1/user/me/`). Empty body.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, field of an unexpected type, or a field breaking its rules: `first_name` / `last_name` 1 to 64 characters, not blank, no control character; `email` a valid address of at most 320 characters; `phone` not a valid number of `phone_country` (or a national number without `phone_country`, 32 characters at most); `phone_country` not an ISO 3166-1 alpha-2 code, or sent without `phone`; `current_password` missing while `email` is sent. The body names the first invalid field.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `phone`: is not a phone number of the selected country")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "`email` was sent with a wrong `current_password`, or the account has no password. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("The current password is incorrect.")
        ),
        (
            status = 429,
            description = "`email` was sent while the user already had their current password checked 5 times in the last 15 minutes, right or wrong (MAIR-746). Nothing is checked nor changed; the `Retry-After` header gives the seconds to wait. Changes without `email` are not limited.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before retrying")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 409,
            description = "`email` is already used by another account (archived ones included). The answer never says whose it is. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Another account already uses this e-mail address.")
        ),
        (
            status = 500,
            description = "Database or Redis error, or the confirmation e-mail could not be sent (then nothing is changed).",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while sending the email.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn patch_me(
    state: web::Data<AppState>,
    view: ValidatedJson<PatchMeView>,
    auth_user: AuthenticatedUser,
    limits: Option<web::Data<RateLimits>>,
    emails: Option<web::Data<AccountEmails>>,
) -> Result<impl Responder, PatchMeError> {
    trigger_patch_me(
        state,
        emails.as_ref().map(web::Data::get_ref),
        view.into_inner(),
        auth_user.id,
        limits.as_ref().map(web::Data::get_ref),
    )
    .await?;
    Ok(HttpResponse::Ok())
}
