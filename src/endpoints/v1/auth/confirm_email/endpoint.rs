use crate::client_ip::client_ip;
use crate::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use crate::database::users::patch_user::PatchUserQueryView;
use crate::email_change;
use crate::emails::AccountEmails;
use crate::endpoints::v1::auth::confirm_email::view::ConfirmEmailView;
use crate::endpoints::validation::ValidatedJson;
use crate::phone::PhoneChange;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfirmEmailError {
    UnknownToken,
    EmailAlreadyUsed,
    DatabaseError,
    RedisError,
    TooManyRequests(u64),
}

impl std::fmt::Display for ConfirmEmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownToken => write!(f, "Unknown token"),
            Self::EmailAlreadyUsed => {
                write!(f, "Another account already uses this e-mail address.")
            }
            Self::DatabaseError | Self::RedisError => {
                write!(
                    f,
                    "The e-mail address could not be changed, please try again later."
                )
            }
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
        }
    }
}

impl ResponseError for ConfirmEmailError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownToken => StatusCode::UNAUTHORIZED,
            Self::EmailAlreadyUsed => StatusCode::CONFLICT,
            Self::DatabaseError | Self::RedisError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn confirm_email_trigger(
    state: &AppState,
    emails: Option<&AccountEmails>,
    token: &str,
) -> Result<(), ConfirmEmailError> {
    let redis = state.get_redis();
    let smart_db = state.get_smart_db();
    let Some((user_id, new_email)) = email_change::take(redis, token).await.map_err(|e| {
        tracing::error!("Confirm e-mail Redis Error: {e}");
        ConfirmEmailError::RedisError
    })?
    else {
        return Err(ConfirmEmailError::UnknownToken);
    };
    let current: GetUserByIdQueryResultView = smart_db
        .fetch_one(&GetUserByIdQueryView::new(user_id))
        .await
        .map_err(|e| {
            tracing::error!("Confirm e-mail DB Error: {e}");
            ConfirmEmailError::DatabaseError
        })?;
    // An account archived since the request must not move to a new address.
    if current.is_archived() {
        return Err(ConfirmEmailError::UnknownToken);
    }
    let write = PatchUserQueryView::new(
        user_id,
        None,
        None,
        Some(&new_email),
        &PhoneChange::Keep,
        None,
    );
    smart_db.execute(write).await.map_err(|e| match e {
        // Taken by another account since the link was sent.
        ApiLibError::Database(DbError::UniqueViolation(_)) => ConfirmEmailError::EmailAlreadyUsed,
        e => {
            tracing::error!("Confirm e-mail DB Error: {e:?}");
            ConfirmEmailError::DatabaseError
        }
    })?;
    // The change is done: a failed alert is logged and never undoes it.
    if let Some(emails) = emails {
        if let Err(e) = emails
            .send_email_changed(current.email(), current.first_name(), &new_email)
            .await
        {
            tracing::error!("Confirm e-mail: the alert e-mail could not be sent: {e}");
        }
    } else {
        tracing::error!("Confirm e-mail: no mailer is registered, no alert was sent");
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    summary = "Confirm a new e-mail address",
    description = "Consumes the token e-mailed to the new address by `PATCH /api/v1/user/me/` and \
                   makes that address the one of the account. The token is single-use and valid one \
                   hour; the account kept its previous address until now. Once the change is made, \
                   an alert e-mail goes to the previous address (a failed alert never undoes the \
                   change). Sessions stay open.\n\n\
                   Public route: the link is opened from the login front, possibly without a \
                   session. Rate limited: 10 requests per 15 minutes per client address (its own \
                   budget), `429` beyond.",
    request_body(
        content = ConfirmEmailView,
        description = "Token received by e-mail at the new address."
    ),
    responses(
        (
            status = 200,
            description = "The account now uses the new address. Empty body."
        ),
        (
            status = 400,
            description = "Malformed JSON body, or `token` empty, too long or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `token`: must not be empty")
        ),
        (
            status = 401,
            description = "Unknown, expired or already used token, or an account archived since the request.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown token")
        ),
        (
            status = 409,
            description = "Another account took the address since the link was sent. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Another account already uses this e-mail address.")
        ),
        (
            status = 429,
            description = "More than 10 token requests in 15 minutes from this client address; `Retry-After` gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before retrying")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database or Redis error.",
            body = String,
            content_type = "text/plain",
            example = json!("The e-mail address could not be changed, please try again later.")
        )
    ),
    tag = "Auth"
)]
#[post("/confirm_email")]
#[allow(clippy::future_not_send)]
pub async fn confirm_email(
    state: web::Data<AppState>,
    body: ValidatedJson<ConfirmEmailView>,
    limits: Option<web::Data<RateLimits>>,
    emails: Option<web::Data<AccountEmails>>,
    request: HttpRequest,
) -> Result<impl Responder, ConfirmEmailError> {
    let ip_address = client_ip(&request);
    if let Some(limits) = limits {
        limits
            .confirm_email_per_ip
            .hit(&ip_key(ip_address))
            .map_err(|refused| ConfirmEmailError::TooManyRequests(refused.retry_after_seconds))?;
    }
    let token = body.into_inner().token().to_string();
    confirm_email_trigger(&state, emails.as_ref().map(web::Data::get_ref), &token).await?;
    Ok(HttpResponse::Ok())
}
