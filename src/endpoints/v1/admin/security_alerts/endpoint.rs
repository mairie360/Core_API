//! `GET /api/v1/admin/security-alerts/`: the stolen-session alerts of the last 30 days, for the
//! Security tab of the administration (MAIR-773).

use crate::database::security_alerts::{AdminSecurityAlert, AdminSecurityAlertsQueryView};
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAlertsError;

impl std::fmt::Display for SecurityAlertsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred while accessing the database.")
    }
}

impl ResponseError for SecurityAlertsError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    get,
    path = "/",
    summary = "Recent security alerts (administration)",
    description = "The alerts of the last 30 days, newest first (500 at most): each use of a session from \
                   another device that closed it (MAIR-773), with the agent, the address and the device of \
                   that use, and when the agent read it at sign-in. Reserved to administrators.",
    responses(
        (status = 200, description = "The alerts.", body = Vec<AdminSecurityAlert>),
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
            status = 500,
            description = "Database error. Safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Admin - Security",
    security(("jwt" = []))
)]
#[get("/")]
pub async fn list_security_alerts(
    _admin: AdminUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, SecurityAlertsError> {
    let alerts: Vec<AdminSecurityAlert> = state
        .get_smart_db()
        .fetch_all(&AdminSecurityAlertsQueryView::new())
        .await
        .map_err(|e| {
            db_error::log("list the security alerts", &e);
            SecurityAlertsError
        })?;
    Ok(HttpResponse::Ok().json(alerts))
}
