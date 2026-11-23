//! Sessions of a user, closed by an administrator (MAIR-584).
//!
//! The administration lists a user's sessions through `GET /api/v1/admin/users/{userId}/`
//! (`sessions`, the latest 100) and closes one or all of them here. A closed session is revoked
//! in the `sessions` table (Core refuses its JWT and its refresh token at once) and published to
//! the revocation list of Redis, read by every other API (MAIR-264).

use crate::database::second_factor::{AccountEmail, GetAccountEmailQueryView};
use crate::database::sessions::revoke_user_sessions::{
    RecentlyRevokedSessionsQueryView, RevokeUserSessionQueryView, RevokeUserSessionsQueryView,
};
use crate::endpoints::admin_guard::AdminUser;
use crate::session_revocation::{max_jwt_lifetime, publish_revoked_sessions};
use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
enum UserSessionsError {
    UnknownUser,
    UnknownSession,
    DatabaseError,
    /// The revocations are committed, but the revocation list of Redis could not be updated.
    RevocationNotPublished,
}

impl std::fmt::Display for UserSessionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::UnknownSession => write!(f, "Unknown session"),
            Self::DatabaseError => write!(f, "Database error occurred"),
            Self::RevocationNotPublished => write!(
                f,
                "The sessions are closed, but their revocation could not be published to every \
                 API: close all the sessions of the user again."
            ),
        }
    }
}

impl ResponseError for UserSessionsError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser | Self::UnknownSession => StatusCode::NOT_FOUND,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RevocationNotPublished => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn database_error(context: &str, e: &ApiLibError) -> UserSessionsError {
    tracing::error!("{context}: {e}");
    UserSessionsError::DatabaseError
}

/// `UnknownUser` unless `user_id` is an active (non-archived) account.
async fn ensure_user(state: &AppState, user_id: u64) -> Result<(), UserSessionsError> {
    match state
        .get_smart_db()
        .fetch_one::<AccountEmail, _>(&GetAccountEmailQueryView::new(user_id))
        .await
    {
        Ok(_) => Ok(()),
        Err(ApiLibError::Database(DbError::NotFound)) => Err(UserSessionsError::UnknownUser),
        Err(e) => Err(database_error("Admin user sessions", &e)),
    }
}

#[utoipa::path(
    delete,
    path = "sessions",
    summary = "Close every session of a user (administration)",
    description = "Revokes every active session of the user (MAIR-584): their JWTs stop working at \
                   once, here and in every API (revocation list), and their refresh tokens are \
                   refused. The user signs in again on each device. Idempotent: a user without \
                   active session answers `204` too, and the call publishes again every session \
                   revoked within the lifetime of a JWT, so it repairs a revocation an earlier call \
                   could not publish. Reserved to administrators; each call is logged with the \
                   administrator and the user.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    responses(
        (status = 204, description = "Every session of the user is closed. Empty body."),
        (
            status = 400,
            description = "`userId` is not an integer.",
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
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 404,
            description = "No active user has this id (unknown or archived).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Database error. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        ),
        (
            status = 503,
            description = "The sessions are revoked in the database, but the revocation could not be published to the other APIs, which would keep accepting the JWTs until they expire. Repeat the request: it publishes again every recently revoked session of the user.",
            body = String,
            content_type = "text/plain",
            example = json!("The sessions are closed, but their revocation could not be published to every API: close all the sessions of the user again.")
        )
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[delete("/sessions")]
pub async fn admin_revoke_user_sessions(
    admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, UserSessionsError> {
    let user_id = path.into_inner();
    ensure_user(&state, user_id).await?;
    let db = state.get_smart_db();
    let revoked: Vec<Uuid> = db
        .fetch_all(&RevokeUserSessionsQueryView::all(user_id))
        .await
        .map_err(|e| database_error("Admin revoke user sessions", &e))?;
    // Published from the table: a session revoked by an earlier call whose publication failed is
    // no longer active, only the table still knows it.
    let to_publish: Vec<Uuid> = db
        .fetch_all(&RecentlyRevokedSessionsQueryView::new(
            user_id,
            max_jwt_lifetime(),
        ))
        .await
        .map_err(|e| database_error("Admin revoke user sessions", &e))?;
    let published = publish_revoked_sessions(state.get_redis(), &to_publish).await;
    tracing::warn!(
        admin_id = admin.id,
        user_id,
        revoked_sessions = revoked.len(),
        "AUDIT: every session of a user closed by an administrator"
    );
    if !published {
        return Err(UserSessionsError::RevocationNotPublished);
    }
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    delete,
    path = "sessions/{sessionId}",
    summary = "Close one session of a user (administration)",
    description = "Revokes one active session of the user (MAIR-584): its JWTs stop working at once, \
                   here and in every API, and its refresh token is refused. The other sessions of \
                   the user stay open. A session that is unknown, belongs to another user or has \
                   already ended answers `404`. Reserved to administrators; each call is logged \
                   with the administrator, the user and the session.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42),
        ("sessionId" = String, Path, description = "Id of the session (`id` of a session of `GET /api/v1/admin/users/{userId}/`).", example = "4f7c2a9e-8d31-4c55-9a0b-3e1f6d2c8b70")
    ),
    responses(
        (status = 204, description = "The session is closed. Empty body."),
        (
            status = 400,
            description = "`userId` is not an integer or `sessionId` is not a session id.",
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
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 404,
            description = "No active user has this id, or the user has no active session with this id.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Unknown user" = (value = json!("Unknown user"))),
                ("Unknown session" = (value = json!("Unknown session")))
            )
        ),
        (
            status = 500,
            description = "Database error. Nothing is changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        ),
        (
            status = 503,
            description = "The session is revoked in the database, but the revocation could not be published to the other APIs. Close all the sessions of the user (`DELETE /api/v1/admin/users/{userId}/sessions`): it publishes again every recently revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("The sessions are closed, but their revocation could not be published to every API: close all the sessions of the user again.")
        )
    ),
    tag = "Admin - Users",
    security(("jwt" = []))
)]
#[delete("/sessions/{sessionId}")]
pub async fn admin_revoke_user_session(
    admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<(u64, Uuid)>,
) -> Result<impl Responder, UserSessionsError> {
    let (user_id, session_id) = path.into_inner();
    ensure_user(&state, user_id).await?;
    let revoked: Vec<Uuid> = state
        .get_smart_db()
        .fetch_all(&RevokeUserSessionQueryView::new(user_id, session_id))
        .await
        .map_err(|e| database_error("Admin revoke user session", &e))?;
    if revoked.is_empty() {
        return Err(UserSessionsError::UnknownSession);
    }
    let published = publish_revoked_sessions(state.get_redis(), &revoked).await;
    tracing::warn!(
        admin_id = admin.id,
        user_id,
        %session_id,
        "AUDIT: a session of a user closed by an administrator"
    );
    if !published {
        return Err(UserSessionsError::RevocationNotPublished);
    }
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    get,
    path = "connection-logs",
    summary = "Sign-in journal of a user (administration)",
    description = "The latest 100 lines of the sign-in journal of the user, newest first (MAIR-773): sign-ins, \
                   refreshes, logouts, expirations, and the uses of a session from another device \
                   (`STOLEN_SESSION_ATTEMPT`, `SUSPICIOUS_IP_CHANGE`, `IP_CHANGE`). Reserved to administrators; \
                   an archived account answers `404` like an unknown one.",
    params(
        ("userId" = u64, Path, description = "Id of the user.", example = 42)
    ),
    responses(
        (status = 200, description = "The journal.", body = Vec<crate::database::security_alerts::ConnectionLog>),
        (
            status = 400,
            description = "`userId` is not an integer.",
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
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 404,
            description = "No active user has this id (unknown or archived).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 500,
            description = "Database error. Safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        )
    ),
    tag = "Admin - Security",
    security(("jwt" = []))
)]
#[actix_web::get("/connection-logs")]
pub async fn admin_get_user_connection_logs(
    _admin: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, UserSessionsError> {
    let user_id = path.into_inner();
    ensure_user(&state, user_id).await?;
    let logs: Vec<crate::database::security_alerts::ConnectionLog> = state
        .get_smart_db()
        .fetch_all(&crate::database::security_alerts::ConnectionLogsQueryView::new(user_id))
        .await
        .map_err(|e| database_error("Admin user connection logs", &e))?;
    Ok(HttpResponse::Ok().json(logs))
}
