//! `POST /api/v1/user/me/password`: the signed-in user changes their own password (MAIR-823,
//! Settings "Sécurité" tab).

use crate::database::auth::user_password::{GetUserPasswordQueryView, UserPassword};
use crate::database::ids::id_to_sql;
use crate::database::sessions::revoke_user_sessions::RevokeUserSessionsQueryView;
use crate::database::users::patch_user::PatchUserQueryView;
use crate::endpoints::db_error;
use crate::endpoints::v1::auth::login::endpoint::is_password_valid;
use crate::endpoints::v1::user::me::password::view::ChangePasswordView;
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::phone::PhoneChange;
use crate::rate_limit::{too_many_requests_response, user_key, RateLimits};
use crate::session_jwt::current_session_id;
use crate::session_revocation::publish_revoked_sessions;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangePasswordError {
    SamePassword,
    WrongPassword,
    TooManyRequests(u64),
    Database,
}

impl std::fmt::Display for ChangePasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SamePassword => write!(
                f,
                "Invalid `new_password`: must differ from the current password"
            ),
            Self::WrongPassword => write!(f, "The current password is incorrect."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for ChangePasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::SamePassword => StatusCode::BAD_REQUEST,
            Self::WrongPassword => StatusCode::FORBIDDEN,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::TooManyRequests(retry_after) => too_many_requests_response(*retry_after),
            _ => HttpResponse::build(self.status_code()).body(self.to_string()),
        }
    }
}

fn database_error(context: &str, e: &ApiLibError) -> ChangePasswordError {
    db_error::log(context, e);
    ChangePasswordError::Database
}

/// Checks the current password, then writes the new hash and revokes every other session of the
/// user in one transaction (MAIR-420). Returns the revoked sessions, to publish after the commit.
async fn change_password(
    state: &AppState,
    user_id: u64,
    keep: Option<Uuid>,
    view: &ChangePasswordView,
) -> Result<Vec<Uuid>, ChangePasswordError> {
    let smart_db = state.get_smart_db();
    let stored: UserPassword = smart_db
        .fetch_one(&GetUserPasswordQueryView::new(user_id))
        .await
        .map_err(|e| database_error("change password: read the current hash", &e))?;
    // An account without a password answers like a wrong one: it sets its first password through
    // the e-mailed link.
    let Some(hash) = stored.password() else {
        return Err(ChangePasswordError::WrongPassword);
    };
    if !is_password_valid(id_to_sql(user_id), view.current_password(), hash).await {
        return Err(ChangePasswordError::WrongPassword);
    }
    if view.new_password() == view.current_password() {
        return Err(ChangePasswordError::SamePassword);
    }
    let new_hash = passwords::hash(view.new_password()).await.map_err(|e| {
        tracing::error!("Change password: hashing error: {e}");
        ChangePasswordError::Database
    })?;
    let query = PatchUserQueryView::new(
        user_id,
        None,
        None,
        None,
        &PhoneChange::Keep,
        Some(new_hash.as_str()),
    );
    let revoke = keep.map_or_else(
        || RevokeUserSessionsQueryView::all(user_id),
        |sid| RevokeUserSessionsQueryView::all_but(user_id, sid),
    );
    let mut tx = smart_db
        .begin()
        .await
        .map_err(|e| database_error("change password: begin", &e))?;
    tx.execute(&query)
        .await
        .map_err(|e| database_error("change password: write the hash", &e))?;
    let revoked: Vec<Uuid> = tx
        .fetch_all(&revoke)
        .await
        .map_err(|e| database_error("change password: revoke the other sessions", &e))?;
    tx.commit()
        .await
        .map_err(|e| database_error("change password: commit", &e))?;
    Ok(revoked)
}

#[utoipa::path(
    post,
    path = "/password",
    operation_id = "change_my_password",
    summary = "Change one's own password",
    description = "Replaces the password of the user carried by the JWT, after checking the \
                   current one. Every other session of the user (other devices) is revoked in the \
                   same transaction and published to the Redis revocation list after the commit; \
                   the session of the request stays open.\n\n\
                   Each call spends the user's budget of current-password checks (5 per 15 \
                   minutes, right or wrong, shared with the e-mail change of \
                   `PATCH /api/v1/user/me/`, MAIR-746) before the hash is compared.",
    request_body(
        content = ChangePasswordView,
        description = "Current and new password.",
        example = json!({ "current_password": "Old-password-42", "new_password": "New-password-2026" })
    ),
    responses(
        (status = 204, description = "Password changed; the other sessions are revoked. Empty body."),
        (
            status = 400,
            description = "Malformed JSON body, unknown field, `new_password` shorter than 8 or longer than 255 characters or holding a control character, `current_password` longer than 255 characters, or `new_password` equal to the current password. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `new_password`: must be at least 8 characters")
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
            description = "Wrong `current_password`, or the account has no password. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("The current password is incorrect.")
        ),
        (
            status = 429,
            description = "The user already had their current password checked 5 times in the last 15 minutes, right or wrong (MAIR-746). Nothing is checked nor changed; the `Retry-After` header gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before retrying")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database error; nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[post("/password")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn change_my_password(
    user: AuthenticatedUser,
    view: ValidatedJson<ChangePasswordView>,
    request: HttpRequest,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
) -> Result<impl Responder, ChangePasswordError> {
    // MAIR-746: every check spends the budget before the hash is compared.
    if let Some(limits) = limits {
        limits
            .password_check_per_user
            .hit(&user_key(user.id))
            .map_err(|refused| {
                tracing::warn!(
                    user_id = user.id,
                    retry_after = refused.retry_after_seconds,
                    "Current password checks rate limited on POST /user/me/password"
                );
                ChangePasswordError::TooManyRequests(refused.retry_after_seconds)
            })?;
    }
    let revoked = change_password(
        &state,
        user.id,
        current_session_id(&request),
        &view.into_inner(),
    )
    .await?;
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    Ok(HttpResponse::NoContent().finish())
}
