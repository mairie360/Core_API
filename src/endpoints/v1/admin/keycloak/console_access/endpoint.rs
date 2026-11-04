use super::view::KeycloakConsoleAccessView;
use crate::endpoints::admin_guard::AdminUser;
use crate::keycloak_console::{
    issue, ConsoleAccessConfig, ConsoleTokenError, ConsoleTokenKind, LINK_TOKEN_TTL_SECONDS,
};
use crate::rate_limit::{too_many_requests_response, user_key, RateLimits};
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{http::StatusCode, post, web, HttpResponse, ResponseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleAccessError {
    /// Keycloak or the Ed25519 session key is not configured on this instance.
    NotConfigured,
    /// The administrator asked for too many links (seconds to wait).
    TooManyRequests(u64),
    /// The link token could not be signed.
    SigningFailed,
}

impl std::fmt::Display for ConsoleAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "Keycloak console access is not configured on this instance."
            ),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::SigningFailed => write!(f, "The console access could not be issued."),
        }
    }
}

impl ResponseError for ConsoleAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::SigningFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(seconds) = self {
            return too_many_requests_response(*seconds);
        }
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Get a temporary link to the Keycloak admin console",
    description = "Issues a temporary access to the Keycloak admin console of the instance realm, \
                   so that a town-hall administrator can manage the agents' Keycloak accounts and \
                   their two-factor authentication without Mairie 360 rebuilding that interface \
                   (MAIR-609). Reserved to administrators (role `Admin`, account not archived).\n\n\
                   Returns a link `https://auth.<domain>/admin-access?token=…` to open in the \
                   browser (new tab). Its token is an `EdDSA` JWT signed by Core (`typ` \
                   `kc-console-link+jwt`, `aud` `keycloak-console`, subject = the administrator), \
                   valid **120 seconds** and **single use**. `GET /admin-access` turns it into a \
                   console cookie valid 30 minutes, limited to the `auth.<domain>` host and the \
                   `/admin` path, then redirects to `/admin/<realm>/console/`; the ingress checks \
                   that cookie on every console request through `GET \
                   /internal/keycloak-console/verify`, which re-checks that the user is still an \
                   active administrator. The token is not a session JWT and is refused by every \
                   API.\n\n\
                   Each call issues a new link (previous ones stay valid until they expire or are \
                   used) and writes an audit log line with the administrator's id, never the \
                   token. Limited to 10 links per administrator per 15 minutes. The Keycloak \
                   sign-in of the console itself is still required: the administrator logs into \
                   the realm with an account holding the `realm-management` roles.",
    responses(
        (
            status = 200,
            description = "Link issued. Open `url` before `expires_in` seconds; it works once.",
            body = KeycloakConsoleAccessView,
            example = json!({
                "url": "https://auth.dev.mairie360-eip.fr/admin-access?token=eyJhbGciOiJFZERTQSIsInR5cCI6ImtjLWNvbnNvbGUtbGluaytqd3QifQ.eyJzdWIiOiIxIn0.c2lnbmF0dXJl",
                "expires_in": 120
            })
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized: No JWT token provided.")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator (Maire, Responsable, User and Guest included).",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 429,
            description = "The administrator already asked for 10 links in the last 15 minutes. Nothing is issued; the `Retry-After` header gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before retrying")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "The link token could not be signed.",
            body = String,
            content_type = "text/plain",
            example = json!("The console access could not be issued.")
        ),
        (
            status = 503,
            description = "Console access is not available on this instance: no Keycloak realm URL (`KEYCLOAK_ISSUER` / `KEYCLOAK_REALM_URL`) or no Ed25519 session key (`JWT_PRIVATE_KEY` + `JWT_PUBLIC_KEY`, MAIR-680).",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak console access is not configured on this instance.")
        )
    ),
    tag = "Admin - Keycloak",
    security(
        ("jwt" = [])
    )
)]
#[post("/console-access")]
pub async fn request_keycloak_console_access(
    admin: AdminUser,
    console: Option<web::Data<ConsoleAccessConfig>>,
    limits: Option<web::Data<RateLimits>>,
) -> Result<HttpResponse, ConsoleAccessError> {
    let console = console.ok_or(ConsoleAccessError::NotConfigured)?;
    if let Some(limits) = limits {
        limits
            .console_access_per_user
            .hit(&user_key(admin.id))
            .map_err(|refused| {
                tracing::warn!(
                    target: "audit",
                    user_id = admin.id,
                    retry_after = refused.retry_after_seconds,
                    "Keycloak console access requests rate limited"
                );
                ConsoleAccessError::TooManyRequests(refused.retry_after_seconds)
            })?;
    }
    let (token, jti) = issue(ConsoleTokenKind::Link, admin.id).map_err(|e| {
        tracing::error!("Keycloak console link not issued: {e}");
        match e {
            ConsoleTokenError::NotConfigured => ConsoleAccessError::NotConfigured,
            _ => ConsoleAccessError::SigningFailed,
        }
    })?;
    tracing::info!(
        target: "audit",
        user_id = admin.id,
        link_id = %jti,
        "Keycloak console access link issued"
    );
    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .json(KeycloakConsoleAccessView {
            url: console.access_link(&token),
            expires_in: LINK_TOKEN_TTL_SECONDS,
        }))
}
