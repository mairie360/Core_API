use crate::client_ip::client_ip;
use crate::database::sessions::revoke_session_by_token_hash::RevokeSessionByTokenHashQueryView;
use crate::endpoints::db_error;
use crate::endpoints::v1::auth::logout::view::LogoutView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::refresh_token;
use crate::session_revocation::publish_revoked_sessions;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutError {
    TooManyRequests(u64),
    Database,
    /// The session is revoked in Postgres but not on the Redis revocation list: the other APIs
    /// would accept its JWTs. Retrying publishes it again.
    RevocationNotPublished,
}

impl std::fmt::Display for LogoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::Database => write!(f, "An error occurred while accessing the database."),
            Self::RevocationNotPublished => {
                write!(f, "The logout could not be completed, please try again.")
            }
        }
    }
}

impl ResponseError for LogoutError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RevocationNotPublished => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::TooManyRequests(retry_after) => too_many_requests_response(*retry_after),
            Self::Database | Self::RevocationNotPublished => {
                HttpResponse::build(self.status_code()).body(self.to_string())
            }
        }
    }
}

/// Revokes the session of `view`'s refresh token and publishes it to the revocation list.
async fn logout_request(view: &LogoutView, state: &AppState) -> Result<(), LogoutError> {
    // Sessions only store the digest of their refresh token (MAIR-390).
    let token_hash = refresh_token::hash(view.refresh_token());
    let revoked: Vec<Uuid> = state
        .get_smart_db()
        .fetch_all(&RevokeSessionByTokenHashQueryView::new(&token_hash))
        .await
        .map_err(|e| {
            db_error::log("logout by refresh token", &e);
            LogoutError::Database
        })?;
    // Other APIs refuse the session's JWTs through the shared revocation list (MAIR-264).
    // A retry publishes it again: the query also returns an already revoked session.
    if publish_revoked_sessions(state.get_redis(), &revoked).await {
        Ok(())
    } else {
        Err(LogoutError::RevocationNotPublished)
    }
}

#[utoipa::path(
    post,
    operation_id = "logout_with_refresh_token",
    path = "",
    summary = "Log out with the refresh token alone",
    description = "Ends the session designated by its refresh token, **without a JWT**: it works \
                   after the access JWT has expired, e.g. an agent signing out of a shared \
                   workstation after a long idle time (MAIR-692). Holding the refresh token is \
                   the proof, as for `POST /api/v1/sessions/refresh`.\n\n\
                   Afterwards the refresh token is refused by `POST /api/v1/sessions/refresh` \
                   (`401`), the session moves to the history with `revoked_at` set, and its JWTs \
                   are refused by Core and, through the Redis revocation list \
                   (`revoked:<session id>`, for `JWT_TIMEOUT`), by the other APIs. The user's \
                   other sessions stay active.\n\n\
                   The answer is the same `204` whether the token matched an active session, an \
                   expired or already revoked one (published again to the revocation list), or \
                   nothing at all: it never tells whether a \
                   token exists, and logging out twice is harmless. Rate limited per client \
                   address with the refresh budget (60 per minute).",
    request_body(
        content = LogoutView,
        description = "Refresh token of the session to end.",
        example = json!({ "refresh_token": "example-refresh-token" })
    ),
    responses(
        (
            status = 204,
            description = "The session of this refresh token, if any was still active, is revoked. Empty body.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing `refresh_token`, or `refresh_token` longer than 512 characters or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `refresh_token`: must not contain control characters")
        ),
        (
            status = 429,
            description = "Too many requests from this client address (refresh budget, 60 per minute). `Retry-After` gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database failure while revoking the session. Safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The session is revoked but could not be published to the Redis revocation list, so the other APIs would still accept its JWTs. Retry the same call: it publishes the revocation again.",
            body = String,
            content_type = "text/plain",
            example = json!("The logout could not be completed, please try again.")
        )
    ),
    tag = "Auth"
)]
#[post("/logout")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn logout(
    body: ValidatedJson<LogoutView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    request: HttpRequest,
) -> Result<impl Responder, LogoutError> {
    if let Some(limits) = limits {
        limits
            .refresh_per_ip
            .hit(&ip_key(client_ip(&request)))
            .map_err(|refused| LogoutError::TooManyRequests(refused.retry_after_seconds))?;
    }
    logout_request(&body.into_inner(), &state).await?;
    Ok(HttpResponse::NoContent().finish())
}
