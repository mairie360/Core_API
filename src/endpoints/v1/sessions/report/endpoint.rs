//! `POST /api/v1/sessions/report`: the front proxy reports a use of the request's session from another
//! device (MAIR-773).
//!
//! The fronts' Next.js proxies see the browser's address and its device proof, which the APIs do not: they
//! compare them with the session (`cnf.jkt`, `net` claims of the JWT) and report what they found through
//! BFF_user, with the session's own JWT. Database logs the use, closes the session for `certain` and
//! `possible`, and queues one alert per session and level; Core publishes the revocation to every API and
//! e-mails the alert (`session_alerts`).

use crate::database::security_alerts::{
    ReportOutcome, ReportSuspiciousSessionQueryView, SuspicionLevel,
};
use crate::emails::AccountEmails;
use crate::endpoints::db_error;
use crate::endpoints::validation::{check_description, Validate, ValidatedJson, ValidationError};
use crate::session_jwt::current_session_id;
use crate::session_revocation::{max_jwt_lifetime, publish_revoked_session};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Longest user agent kept with a report (`device_info` of the sessions).
pub const MAX_DEVICE_INFO_LENGTH: usize = 255;

/// What the front proxy saw.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReportSessionView {
    pub level: SuspicionLevel,
    /// Address of the browser that sent the request, as the proxy saw it.
    #[schema(value_type = String, example = "203.0.113.7")]
    pub ip_address: std::net::IpAddr,
    /// User agent of that browser (at most 255 characters, may be empty).
    #[schema(
        max_length = 255,
        example = "Mozilla/5.0 (X11; Linux x86_64) Firefox/140.0"
    )]
    #[serde(default)]
    pub device_info: String,
}

impl Validate for ReportSessionView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_description("device_info", &self.device_info, MAX_DEVICE_INFO_LENGTH)
    }
}

impl std::fmt::Display for ReportSessionView {
    // Never the address nor the user agent.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReportSessionView {{ level: {:?} }}", self.level)
    }
}

/// Answer of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReportSessionResponse {
    /// The session was active and is now closed: the front must send the agent back to the sign-in.
    #[schema(example = true)]
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportSessionError {
    NoSession,
    UnknownSession,
    Database,
}

impl std::fmt::Display for ReportSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSession => write!(f, "The JWT carries no session id."),
            Self::UnknownSession => write!(f, "Unknown session"),
            Self::Database => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for ReportSessionError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NoSession => StatusCode::BAD_REQUEST,
            Self::UnknownSession => StatusCode::NOT_FOUND,
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    post,
    path = "report",
    summary = "Report a use of the session from another device",
    description = "Called by the front proxies (through BFF_user) with the JWT of the session they checked \
                   (MAIR-773). `certain`: the session is bound to a device key and the request came \
                   without a valid proof. `possible`: an unbound session was used from another network \
                   than the one it was opened from. `ip_change`: a bound session changed network with a \
                   valid proof (logged, the session stays open and follows the new address).\n\n\
                   `certain` and `possible` close the session (`revoked: true` when it was still active, \
                   its JWTs are refused by every API at once) and queue one alert per session and level: \
                   the agent and the administrators are e-mailed, and the Login screen shows it at the \
                   next sign-in (`GET /api/v1/user/me/security-alert/`).",
    request_body(
        content = ReportSessionView,
        example = json!({ "level": "ip_change", "ip_address": "203.0.113.7", "device_info": "Mozilla/5.0 (X11; Linux x86_64) Firefox/140.0" })
    ),
    responses(
        (
            status = 200,
            description = "The use is recorded.",
            body = ReportSessionResponse,
            example = json!({ "revoked": true })
        ),
        (
            status = 400,
            description = "Invalid body (unknown `level`, `ip_address` that is not an address, `device_info` too long or with control characters), or a JWT without session id.",
            body = String,
            content_type = "text/plain",
            example = json!("The JWT carries no session id.")
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
            description = "The session of the JWT no longer exists.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown session")
        ),
        (
            status = 500,
            description = "Database error. Nothing is recorded; safe to retry.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Sessions",
    security(("jwt" = []))
)]
#[post("/report")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn report_session_use(
    user: AuthenticatedUser,
    request: HttpRequest,
    body: ValidatedJson<ReportSessionView>,
    state: web::Data<AppState>,
    emails: Option<web::Data<AccountEmails>>,
) -> Result<impl Responder, ReportSessionError> {
    let session_id = current_session_id(&request).ok_or(ReportSessionError::NoSession)?;
    let report = body.into_inner();
    let outcome: ReportOutcome = state
        .get_smart_db()
        .fetch_one(&ReportSuspiciousSessionQueryView::new(
            session_id,
            user.id,
            report.level,
            report.ip_address,
            &report.device_info,
        ))
        .await
        .map_err(|e| {
            db_error::log("report a suspicious use of a session", &e);
            ReportSessionError::Database
        })?;
    if !outcome.known {
        return Err(ReportSessionError::UnknownSession);
    }
    if outcome.revoked {
        if let Err(e) =
            publish_revoked_session(state.get_redis(), session_id, max_jwt_lifetime()).await
        {
            tracing::error!(
                %session_id,
                "[CRITICAL] Reported session revoked but not published to Redis: {e}"
            );
        }
    }
    if outcome.alert_id.is_some() {
        tracing::warn!(
            target: "audit",
            event = "suspicious_session_use",
            %session_id,
            user_id = user.id,
            level = report.level.as_sql(),
            "Session used from another device: closed and alert queued"
        );
        crate::session_alerts::spawn_delivery(&state, emails.as_ref());
    }
    Ok(HttpResponse::Ok().json(ReportSessionResponse {
        revoked: outcome.revoked,
    }))
}
