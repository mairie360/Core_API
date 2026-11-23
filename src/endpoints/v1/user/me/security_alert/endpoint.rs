//! The stolen-session alert of the caller (MAIR-773): the Login screen reads the latest alert the agent
//! has not read yet right after the sign-in ("Activité inhabituelle"), then marks it read.

use crate::database::security_alerts::{
    AcknowledgeSecurityAlertQueryView, PendingSecurityAlertQueryView, SecurityAlert,
};
use crate::endpoints::db_error;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityAlertError {
    UnknownAlert,
    Database,
}

impl std::fmt::Display for SecurityAlertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownAlert => write!(f, "Unknown alert"),
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for SecurityAlertError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownAlert => StatusCode::NOT_FOUND,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    get,
    path = "/security-alert/",
    summary = "Pending security alert of the caller",
    description = "The latest alert of a session of the caller used from another device that the caller \
                   has not read yet (MAIR-773): `certain` when the session was bound to a device and the \
                   request came without its proof, `possible` when it was used from another network. The \
                   session was closed when the use was reported. The Login screen shows it right after the \
                   sign-in, then marks it read. `204` when there is none.",
    responses(
        (status = 200, description = "The pending alert.", body = SecurityAlert),
        (status = 204, description = "No pending alert. Empty body."),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error. Safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "User",
    security(("jwt" = []))
)]
#[get("/security-alert/")]
pub async fn get_my_security_alert(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, SecurityAlertError> {
    let alerts: Vec<SecurityAlert> = state
        .get_smart_db()
        .fetch_all(&PendingSecurityAlertQueryView::new(user.id))
        .await
        .map_err(|e| {
            db_error::log("read the pending security alert", &e);
            SecurityAlertError::Database
        })?;
    Ok(alerts.into_iter().next().map_or_else(
        || HttpResponse::NoContent().finish(),
        |alert| HttpResponse::Ok().json(alert),
    ))
}

#[utoipa::path(
    post,
    path = "/security-alert/{alertId}/acknowledge/",
    summary = "Mark a security alert of the caller as read",
    description = "Marks an alert of the caller as read (MAIR-773), so the Login screen no longer shows it. \
                   An unknown alert, another account's or one already read answers `404`.",
    params(
        ("alertId" = u64, Path, description = "Id of the alert (`id` of `GET /api/v1/user/me/security-alert/`).", example = 12)
    ),
    responses(
        (status = 204, description = "The alert is read. Empty body."),
        (
            status = 400,
            description = "`alertId` is not an integer.",
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
            description = "No unread alert of the caller has this id.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown alert")
        ),
        (
            status = 500,
            description = "Database error. Safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "User",
    security(("jwt" = []))
)]
#[post("/security-alert/{alertId}/acknowledge/")]
pub async fn acknowledge_my_security_alert(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, SecurityAlertError> {
    let acknowledged: Vec<i64> = state
        .get_smart_db()
        .fetch_all(&AcknowledgeSecurityAlertQueryView::new(
            user.id,
            path.into_inner(),
        ))
        .await
        .map_err(|e| {
            db_error::log("acknowledge a security alert", &e);
            SecurityAlertError::Database
        })?;
    if acknowledged.is_empty() {
        return Err(SecurityAlertError::UnknownAlert);
    }
    Ok(HttpResponse::NoContent().finish())
}
