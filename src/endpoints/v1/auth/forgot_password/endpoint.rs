use crate::client_ip::client_ip;
use crate::database::auth::reset_recipient::{
    ResetRecipientQueryResultView, ResetRecipientQueryView,
};
use crate::emails::AccountEmails;
use crate::endpoints::v1::auth::forgot_password::view::ForgotPasswordView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{email_key, ip_key, too_many_requests_response, RateLimits};
use crate::redis_keys::{set_token, FORGOT_PASSWORD_TTL_SECONDS};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

/// Only infrastructure failures are reported: whether the e-mail matches an account, is already
/// waiting for a reset or is not eligible never changes the response (no account enumeration).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResetPasswordError {
    Database,
    Mail,
    Redis,
    TooManyRequests(u64),
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Mail => {
                write!(f, "An error occurred while sending the email.")
            }
            Self::Redis => {
                write!(f, "An error occurred while accessing Redis.")
            }
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// First name of the account a reset e-mail may be sent to: a non-archived account uses `email`.
/// `None` when there is none. Activated accounts are eligible too (MAIR-390): before, only
/// accounts still waiting for their first connection were, so a user who forgot their password
/// silently received nothing.
async fn reset_recipient(
    smart_db: &SmartDatabase,
    email: &str,
) -> Result<Option<String>, ResetPasswordError> {
    smart_db
        .fetch_all::<ResetRecipientQueryResultView, _>(&ResetRecipientQueryView::new(email))
        .await
        .map(|rows| rows.first().map(|row| row.first_name().to_string()))
        .map_err(|e| {
            tracing::error!("Forgot password DB Error: {e}");
            ResetPasswordError::Database
        })
}

async fn trigger(
    state: &AppState,
    emails: Option<&AccountEmails>,
    email: &str,
    first_name: &str,
) -> Result<(), ResetPasswordError> {
    let Some(emails) = emails else {
        tracing::error!("Forgot password: no mailer is registered, the e-mail cannot be sent");
        return Err(ResetPasswordError::Mail);
    };
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    let token_key = format!("{email}/forgot_password_token");
    let email_key = format!("{token}/forgot_password_email");
    set_token(redis, &token_key, &token, FORGOT_PASSWORD_TTL_SECONDS)
        .await
        .map_err(|e| {
            tracing::error!("Redis Error: {e}");
            ResetPasswordError::Redis
        })?;
    set_token(redis, &email_key, email, FORGOT_PASSWORD_TTL_SECONDS)
        .await
        .map_err(|e| {
            tracing::error!("Redis Error: {e}");
            ResetPasswordError::Redis
        })?;
    if let Err(e) = emails.send_password_reset(email, first_name, &token).await {
        tracing::error!("Forgot password: the e-mail could not be sent: {e}");
        // Nobody received this token: drop it, otherwise the pending reset would silence every
        // new request for this address until it expires.
        for key in [&token_key, &email_key] {
            if let Err(e) = redis.secure_delete(key).await {
                tracing::warn!("Forgot password: could not drop an unsent token: {e}");
            }
        }
        return Err(ResetPasswordError::Mail);
    }
    Ok(())
}

async fn forgot_password_trigger(
    state: web::Data<AppState>,
    emails: Option<&AccountEmails>,
    view: ForgotPasswordView,
) -> Result<(), ResetPasswordError> {
    let token = state
        .get_redis()
        .secure_get::<String>(&format!("{}/forgot_password_token", view.email()))
        .await;
    // A reset is already waiting for this address: answer as if a new e-mail had been sent.
    if matches!(token, Ok(Some(_))) {
        return Ok(());
    }

    if let Some(first_name) = reset_recipient(state.get_smart_db(), view.email()).await? {
        trigger(&state, emails, view.email(), &first_name).await?;
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    summary = "Request a password reset",
    description = "If the e-mail matches an account, generates a one-time reset token valid \
                   **15 minutes**, stores it in Redis and e-mails the user the Resend template \
                   `account-password-reset`, sent from `no-reply@<EMAIL_DOMAIN>`. The e-mail \
                   holds a link to the login front, `<PUBLIC_URL>/reset-password?token=<token>`: \
                   that page sends the token to `POST /api/v1/auth/reset_password`.\n\n\
                   Public route: `JwtMiddleware` lets everything under `/auth` through.\n\n\
                   To avoid revealing which e-mails have an account, the answer is **always the \
                   same empty `200`**, whether the address is unknown, already has a pending \
                   reset (no second e-mail is sent until the first token is used) or matches an \
                   account. The token is never returned in the response. Only infrastructure \
                   failures answer `500`.\n\n\
                   Every non-archived account is eligible, whether it was activated or not; an \
                   archived account is treated like an unknown address.\n\n\
                   Rate limited: 10 requests per 15 minutes per client address and 3 per hour per \
                   e-mail address; beyond that the answer is `429`.",
    request_body(
        content = ForgotPasswordView,
        description = "E-mail address of the account to reset.",
        example = json!({ "email": "jean.dupont@mairie360.fr" })
    ),
    responses(
        (
            status = 200,
            description = "Request accepted. Empty body. Does not tell whether an e-mail was sent.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing `email`, or `email` that is not a valid address of at most 320 characters.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `email`: must be a valid e-mail address")
        ),
        (
            status = 429,
            description = "Too many requests from this client address or for this e-mail address. The `Retry-After` header gives the number of seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(
                ("Retry-After" = u64, description = "Seconds to wait before the next request.")
            ),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database or Redis failure, or Resend refused or could not receive the e-mail. The unsent token is dropped, so the request can be retried at once.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while sending the email.")
        )
    ),
    tag = "Auth"
)]
#[post("/forgot_password")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn forgot_password(
    state: web::Data<AppState>,
    body: ValidatedJson<ForgotPasswordView>,
    limits: Option<web::Data<RateLimits>>,
    emails: Option<web::Data<AccountEmails>>,
    request: HttpRequest,
) -> Result<impl Responder, ResetPasswordError> {
    let view = body.into_inner();
    if let Some(limits) = limits {
        limits
            .forgot_password_per_ip
            .hit(&ip_key(client_ip(&request)))
            .and_then(|()| {
                limits
                    .forgot_password_per_email
                    .hit(&email_key(view.email()))
            })
            .map_err(|refused| ResetPasswordError::TooManyRequests(refused.retry_after_seconds))?;
    }
    forgot_password_trigger(state, emails.as_ref().map(web::Data::get_ref), view).await?;
    Ok(HttpResponse::Ok())
}
