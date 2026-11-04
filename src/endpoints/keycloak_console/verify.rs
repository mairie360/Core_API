use super::is_active_admin;
use crate::keycloak_console::{verify, ConsoleTokenKind, CONSOLE_COOKIE_NAME};
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{get, http::StatusCode, web, HttpRequest, HttpResponse, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    /// No valid console cookie, or its user is no longer an active administrator.
    Unauthorized,
    /// The administrator could not be checked in the database.
    Internal,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(
                f,
                "Keycloak console access expired or missing. Ask for a new link from the Mairie 360 administration."
            ),
            Self::Internal => write!(f, "The console access could not be checked."),
        }
    }
}

impl ResponseError for VerifyError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(CacheControl(vec![CacheDirective::NoStore]))
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

#[utoipa::path(
    get,
    path = "internal/keycloak-console/verify",
    summary = "Check the Keycloak console cookie (ForwardAuth)",
    description = "Called by the Traefik `ForwardAuth` middleware in front of the Keycloak realm \
                   admin console (`/admin/<realm>/console/` and `/admin/realms/<realm>/…`) for \
                   every request the browser sends there (MAIR-609, MAIR-610), with the browser's \
                   headers. Reached in the cluster only: the ingress never routes `/internal` \
                   publicly.\n\n\
                   Reads the console cookie `__Secure-mairie360-kc-console` set by `GET \
                   /admin-access`, checks it (Core's `EdDSA` signature, `typ` \
                   `kc-console-cookie+jwt`, `aud` `keycloak-console-cookie`, expiry: 30 minutes) \
                   and re-checks in the database, on every call, that its user is still an \
                   administrator whose account is not archived. `200` lets the request through to \
                   Keycloak; any other status is sent back to the browser instead. A session JWT \
                   or a link token in the cookie is refused. Nothing is written, no audit line on \
                   success.",
    params(
        ("__Secure-mairie360-kc-console" = Option<String>, Cookie, description = "Console cookie set by `GET /admin-access`.")
    ),
    responses(
        (
            status = 200,
            description = "Valid cookie of an active administrator: the console request may go through.",
            body = String,
            content_type = "text/plain",
            example = json!("OK")
        ),
        (
            status = 401,
            description = "Cookie missing, malformed, forged, of another kind or expired, or its user lost the `Admin` role or was archived.",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak console access expired or missing. Ask for a new link from the Mairie 360 administration.")
        ),
        (
            status = 500,
            description = "The administrator could not be checked in the database: the request is refused.",
            body = String,
            content_type = "text/plain",
            example = json!("The console access could not be checked.")
        )
    ),
    tag = "Keycloak console"
)]
#[get("/internal/keycloak-console/verify")]
// actix-web runs each worker on a single-threaded runtime: handler futures need not be Send.
#[allow(clippy::future_not_send)]
pub async fn verify_keycloak_console_access(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, VerifyError> {
    let cookie = req
        .cookie(CONSOLE_COOKIE_NAME)
        .ok_or(VerifyError::Unauthorized)?;
    let claims = verify(ConsoleTokenKind::Cookie, cookie.value()).map_err(|e| {
        tracing::warn!(reason = %e, "Keycloak console cookie refused");
        VerifyError::Unauthorized
    })?;
    let user_id = claims.user_id().ok_or(VerifyError::Unauthorized)?;
    if !is_active_admin(&state, user_id)
        .await
        .map_err(|_| VerifyError::Internal)?
    {
        tracing::warn!(
            target: "audit",
            user_id,
            "Keycloak console request refused: no longer an administrator"
        );
        return Err(VerifyError::Unauthorized);
    }
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .content_type("text/plain; charset=utf-8")
        .body("OK"))
}
