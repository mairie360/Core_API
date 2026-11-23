//! `DELETE /api/v1/sessions/{sessionId}/`: the caller ends one of their other sessions (another
//! device) by its id (MAIR-823, Settings "Sessions" tab).

use crate::database::sessions::revoke_session_by_id::RevokeSessionByIdQueryView;
use crate::endpoints::db_error;
use crate::session_jwt::current_session_id;
use crate::session_revocation::publish_revoked_sessions;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevokeByIdError {
    NotFound,
    CurrentSession,
    Database,
}

impl std::fmt::Display for RevokeByIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Session not found."),
            Self::CurrentSession => write!(
                f,
                "This is the current session: use POST /api/v1/sessions/logout to end it."
            ),
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for RevokeByIdError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::CurrentSession => StatusCode::CONFLICT,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    delete,
    path = "{sessionId}/",
    operation_id = "revoke_session_by_id",
    summary = "End one of your other sessions",
    description = "Revokes one active session of the user carried by the JWT, identified by its id \
                   (`id` of `GET /api/v1/sessions/`): typically a device the user does not \
                   recognise. The session moves to the history with `revoked_at` set, its refresh \
                   token stops working, and its id is written to the Redis revocation list \
                   (`revoked:<session id>`, for `JWT_TIMEOUT`) so every API refuses its JWTs.\n\n\
                   The session of the request itself cannot be ended here (`409`): use \
                   `POST /api/v1/sessions/logout`. A session of another user, an unknown id or a \
                   session already revoked or expired answers the same `404`.",
    params(
        ("sessionId" = String, Path, description = "Id of the session to end.", example = "0b6f2a54-3c1d-4e0a-9f41-6d2b1c7e8a90")
    ),
    responses(
        (status = 204, description = "Session revoked. Empty body."),
        (
            status = 400,
            description = "The path id is not a session id.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid path: ids are integers.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "No active session of the caller has this id (unknown, another user's, already revoked or expired).",
            body = String,
            content_type = "text/plain",
            example = json!("Session not found.")
        ),
        (
            status = 409,
            description = "The id is the session of the request: log out instead.",
            body = String,
            content_type = "text/plain",
            example = json!("This is the current session: use POST /api/v1/sessions/logout to end it.")
        ),
        (
            status = 500,
            description = "Database error while revoking the session. Safe to retry.",
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
#[delete("/{session_id}/")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn revoke_session_by_id(
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RevokeByIdError> {
    let session_id = path.into_inner();
    if current_session_id(&request) == Some(session_id) {
        return Err(RevokeByIdError::CurrentSession);
    }
    let revoked: Vec<Uuid> = state
        .get_smart_db()
        .fetch_all(&RevokeSessionByIdQueryView::new(user.id, session_id))
        .await
        .map_err(|e| {
            db_error::log("revoke a session by id", &e);
            RevokeByIdError::Database
        })?;
    if revoked.is_empty() {
        return Err(RevokeByIdError::NotFound);
    }
    // Failures are logged as [CRITICAL] by the helper: Postgres already refuses the session.
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    Ok(HttpResponse::NoContent().finish())
}
