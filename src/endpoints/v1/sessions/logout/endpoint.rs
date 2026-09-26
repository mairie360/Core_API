use crate::database::sessions::revoke_current_session::RevokeCurrentSessionQueryView;
use crate::session_jwt::decode_session_jwt;
use crate::session_revocation::publish_revoked_session;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::jwt_manager::get_jwt_from_request;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogoutError {
    Database,
    Redis,
}

impl std::fmt::Display for LogoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database => write!(f, "An error occurred while accessing the database."),
            Self::Redis => write!(f, "An error occurred while accessing Redis."),
        }
    }
}

impl ResponseError for LogoutError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Database | Self::Redis => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    post,
    path = "logout",
    summary = "Log out: revoke the current session",
    description = "Revokes the session the request's JWT belongs to, from the JWT alone (no \
                   refresh token needed). Afterwards the same JWT is refused with `401` on every \
                   Core route, and the session's refresh token no longer works with \
                   `POST /api/v1/sessions/refresh`. The user's other sessions (other devices) \
                   stay active; to close one of them, use `POST /api/v1/sessions/revoke`.\n\n\
                   Logging out again with the same JWT answers `401`, like any other route: the \
                   JWT of a revoked session is refused before this route runs. A JWT issued \
                   before this route existed carries no session id: the call answers `204` \
                   without revoking anything, and that JWT simply expires after \
                   `JWT_TIMEOUT`.\n\n\
                   The session is also written to the Redis revocation list \
                   (`revoked:<session id>`, until the JWT expires): the other APIs refuse the JWT \
                   once they run a `mairie360_api_lib` version that checks it.",
    responses(
        (
            status = 204,
            description = "Session revoked. Empty body.",
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, or invalid, expired or revoked JWT (including a second logout with the same JWT).",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database or Redis failure while revoking the session. Safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[post("/logout")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn logout(
    user: AuthenticatedUser,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, LogoutError> {
    let claims = get_jwt_from_request(&request).and_then(|jwt| decode_session_jwt(&jwt).ok());
    let session = claims
        .as_ref()
        .and_then(|claims| Some((claims.session_id()?, claims.remaining_lifetime())));

    if let Some((session_id, remaining_lifetime)) = session {
        state
            .get_smart_db()
            .execute(RevokeCurrentSessionQueryView::new(session_id, user.id))
            .await
            .map_err(|e| {
                eprintln!("Logout DB Error: {e}");
                LogoutError::Database
            })?;
        // Other APIs refuse the JWT through the shared revocation list (MAIR-264). A failure is
        // reported: logging out again retries the write.
        publish_revoked_session(state.get_redis(), session_id, remaining_lifetime)
            .await
            .map_err(|e| {
                eprintln!("Logout Redis Error: {e}");
                LogoutError::Redis
            })?;
    }

    Ok(HttpResponse::NoContent().finish())
}
