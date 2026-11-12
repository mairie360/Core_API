use super::is_active_admin;
use crate::keycloak_console::{
    issue, used_link_key, verify, ConsoleAccessConfig, ConsoleTokenError, ConsoleTokenKind,
    CONSOLE_COOKIE_NAME, CONSOLE_COOKIE_PATH, CONSOLE_COOKIE_TTL_SECONDS,
};
use actix_web::cookie::{time::Duration, Cookie, SameSite};
use actix_web::http::header::{CacheControl, CacheDirective, LOCATION, REFERRER_POLICY};
use actix_web::{get, http::StatusCode, web, HttpResponse, ResponseError};
use mairie360_api_lib::state::AppState;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

/// Longest token accepted: a link token is about 400 characters.
const MAX_TOKEN_LENGTH: usize = 4096;

/// Query string of `GET /admin-access`.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct AdminAccessQuery {
    /// The link token returned in `url` by `POST /api/v1/admin/keycloak/console-access`.
    /// Single use, valid 120 seconds.
    #[param(example = "<link token: a signed JWT>")]
    #[schema(example = "<link token: a signed JWT>")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminAccessError {
    /// Missing, malformed, forged, expired or already used token.
    InvalidLink,
    /// The token is genuine but its user is no longer an active administrator.
    NotAdmin,
    /// Keycloak, the session keys or Redis are not available.
    Unavailable,
    /// Database failure or cookie signing failure.
    Internal,
}

impl std::fmt::Display for AdminAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLink => write!(
                f,
                "This console access link is invalid, expired or already used. Ask for a new one from the Mairie 360 administration."
            ),
            Self::NotAdmin => write!(f, "Forbidden: User is not an admin."),
            Self::Unavailable => write!(
                f,
                "Keycloak console access is not available on this instance."
            ),
            Self::Internal => write!(f, "The console access could not be opened."),
        }
    }
}

impl ResponseError for AdminAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidLink => StatusCode::UNAUTHORIZED,
            Self::NotAdmin => StatusCode::FORBIDDEN,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
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

/// The console cookie: host-only (no `Domain`, so only `auth.<domain>` gets it), `/admin` only,
/// `HttpOnly`, `Secure`, `SameSite=Strict`, as long as the token it holds.
#[must_use]
pub fn console_cookie(value: String) -> Cookie<'static> {
    Cookie::build(CONSOLE_COOKIE_NAME, value)
        .path(CONSOLE_COOKIE_PATH)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(Duration::seconds(
            i64::try_from(CONSOLE_COOKIE_TTL_SECONDS).unwrap_or(i64::MAX),
        ))
        .finish()
}

#[utoipa::path(
    get,
    path = "admin-access",
    summary = "Open the Keycloak admin console with a temporary link",
    description = "Browser route, served under `https://auth.<domain>/admin-access` by the \
                   ingress (MAIR-609, MAIR-610): the administrator lands here by following the \
                   link of `POST /api/v1/admin/keycloak/console-access`. No JWT: the link token \
                   is the credential.\n\n\
                   Checks the token (Core's `EdDSA` signature, `typ` `kc-console-link+jwt`, `aud` \
                   `keycloak-console`, expiry) and records it as used in Redis until it expires, \
                   so a second visit is refused (single use), then re-checks that its user is \
                   still an administrator whose account is not archived. On success, sets the \
                   console cookie `__Secure-mairie360-kc-console` (`Path=/admin`, no `Domain`, \
                   `HttpOnly`, `Secure`, `SameSite=Strict`, 30 minutes) and redirects to the realm \
                   console `/admin/<realm>/console/`. The token is consumed even when the user \
                   turns out not to be an administrator any more. Session JWTs and console \
                   cookies are refused here.\n\n\
                   Every answer carries `Cache-Control: no-store`; the redirect also carries \
                   `Referrer-Policy: no-referrer` so the token does not leak to the console.",
    params(AdminAccessQuery),
    responses(
        (
            status = 303,
            description = "Access granted: the console cookie is set and the browser is sent to the realm console.",
            headers(
                ("Location" = String, description = "Path of the realm admin console, e.g. `/admin/mairie360/console/`"),
                ("Set-Cookie" = String, description = "`__Secure-mairie360-kc-console=<signed token>; Path=/admin; Max-Age=1800; HttpOnly; Secure; SameSite=Strict`")
            )
        ),
        (
            status = 401,
            description = "`token` missing, malformed, forged (not signed by Core), of another kind (session JWT, console cookie), expired or already used.",
            body = String,
            content_type = "text/plain",
            example = json!("This console access link is invalid, expired or already used. Ask for a new one from the Mairie 360 administration.")
        ),
        (
            status = 403,
            description = "The token is genuine but its user lost the `Admin` role or was archived since it was issued.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 500,
            description = "The administrator could not be checked in the database, or the cookie could not be signed.",
            body = String,
            content_type = "text/plain",
            example = json!("The console access could not be opened.")
        ),
        (
            status = 503,
            description = "Console access is not configured on this instance (Keycloak realm URL or Ed25519 session key missing), or Redis could not record the token as used.",
            body = String,
            content_type = "text/plain",
            example = json!("Keycloak console access is not available on this instance.")
        )
    ),
    tag = "Keycloak console"
)]
#[get("/admin-access")]
pub async fn open_keycloak_console(
    query: web::Query<AdminAccessQuery>,
    state: web::Data<AppState>,
    console: Option<web::Data<ConsoleAccessConfig>>,
) -> Result<HttpResponse, AdminAccessError> {
    let console = console.ok_or(AdminAccessError::Unavailable)?;
    let token = query
        .token
        .as_deref()
        .filter(|token| !token.is_empty() && token.len() <= MAX_TOKEN_LENGTH)
        .ok_or(AdminAccessError::InvalidLink)?;
    let claims = verify(ConsoleTokenKind::Link, token).map_err(|e| {
        tracing::warn!(target: "audit", reason = %e, "Keycloak console link refused");
        if e == ConsoleTokenError::NotConfigured {
            AdminAccessError::Unavailable
        } else {
            AdminAccessError::InvalidLink
        }
    })?;
    let user_id = claims.user_id().ok_or(AdminAccessError::InvalidLink)?;
    // Single use: the first visit records the token id a little past the token's expiry
    // (`SET NX EX`), so no replay fits between the end of the marker and the end of the token.
    let first_use = state
        .get_redis()
        .secure_set_ex(&used_link_key(claims.jti()), "1", claims.used_marker_ttl())
        .await
        .map_err(|e| {
            tracing::error!("Keycloak console link not recorded in Redis: {e}");
            AdminAccessError::Unavailable
        })?;
    if !first_use {
        tracing::warn!(
            target: "audit",
            user_id,
            link_id = %claims.jti(),
            "Keycloak console link replayed"
        );
        return Err(AdminAccessError::InvalidLink);
    }
    if !is_active_admin(&state, user_id)
        .await
        .map_err(|_| AdminAccessError::Internal)?
    {
        tracing::warn!(
            target: "audit",
            user_id,
            "Keycloak console link refused: no longer an administrator"
        );
        return Err(AdminAccessError::NotAdmin);
    }
    let (cookie, _) = issue(ConsoleTokenKind::Cookie, user_id).map_err(|e| {
        tracing::error!("Keycloak console cookie not issued: {e}");
        if e == ConsoleTokenError::NotConfigured {
            AdminAccessError::Unavailable
        } else {
            AdminAccessError::Internal
        }
    })?;
    tracing::info!(
        target: "audit",
        user_id,
        link_id = %claims.jti(),
        "Keycloak console session opened"
    );
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, console.console_path()))
        .insert_header(CacheControl(vec![CacheDirective::NoStore]))
        .insert_header((REFERRER_POLICY, "no-referrer"))
        .cookie(console_cookie(cookie))
        .finish())
}
