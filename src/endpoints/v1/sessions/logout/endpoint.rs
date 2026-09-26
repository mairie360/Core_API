use crate::database::sessions::revoke_current_session::RevokeCurrentSessionQueryView;
use crate::session_jwt::decode_session_jwt;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::jwt_manager::get_jwt_from_request;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogoutError {
    Database,
}

impl std::fmt::Display for LogoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for LogoutError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
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
                   Idempotent: logging out again with the same JWT answers `204` again (this is \
                   the only route that still accepts the JWT of a revoked session, and it does \
                   nothing with it). A JWT issued before this route \
                   existed carries no session id: the call answers `204` without revoking \
                   anything, and that JWT simply expires after `JWT_TIMEOUT`.\n\n\
                   Other APIs (Project, Calendar, ...) keep accepting a revoked JWT until it \
                   expires: only Core checks the session behind a JWT.",
    responses(
        (
            status = 204,
            description = "Session revoked (or already revoked). Empty body.",
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, or invalid or expired JWT.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database failure while revoking the session.",
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
    let session_id = get_jwt_from_request(&request)
        .and_then(|jwt| decode_session_jwt(&jwt).ok())
        .and_then(|claims| claims.session_id());

    if let Some(session_id) = session_id {
        state
            .get_smart_db()
            .execute(RevokeCurrentSessionQueryView::new(session_id, user.id))
            .await
            .map_err(|e| {
                eprintln!("Logout DB Error: {e}");
                LogoutError::Database
            })?;
    }

    Ok(HttpResponse::NoContent().finish())
}
