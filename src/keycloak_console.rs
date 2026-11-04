//! Temporary access to the Keycloak admin console of the instance realm (MAIR-609).
//!
//! The realm admin console (`https://auth.<domain>/admin/<realm>/console/`) is not open on the
//! public auth host (MAIR-610): it sits behind a Traefik `ForwardAuth` that asks Core whether the
//! browser may reach it. Only town-hall administrators (role `Admin`, account not archived) get
//! through, in three steps:
//!
//! 1. `POST /api/v1/admin/keycloak/console-access` (admin only) issues a **link token**: an
//!    Ed25519 JWT signed with Core's session key, valid [`LINK_TOKEN_TTL_SECONDS`], single use,
//!    and returns `https://auth.<domain>/admin-access?token=…`.
//! 2. `GET /admin-access?token=…` (served under `auth.<domain>` by the ingress) checks the link
//!    token, records its `jti` in Redis so it cannot be replayed, re-checks that the user is still
//!    an administrator, then sets the **console cookie** [`CONSOLE_COOKIE_NAME`] (another JWT, valid
//!    [`CONSOLE_COOKIE_TTL_SECONDS`], `Path=/admin`, host-only, `HttpOnly`, `Secure`,
//!    `SameSite=Strict`) and redirects to the console.
//! 3. `GET /internal/keycloak-console/verify` (called by the `ForwardAuth` for every console
//!    request, never routed publicly) checks the cookie and re-checks the role in the database.
//!
//! Both tokens reuse the session signing key of MAIR-680 (`JWT_PRIVATE_KEY`) but each has its
//! own `aud` and `typ`: a session JWT is neither a link token nor a console cookie, and neither
//! of them is a session JWT (the session verifier requires `aud` = `mairie360` and a `role`
//! claim). Without the Ed25519 key (HS256 transition) console access is not available: an HS256
//! session token does not check its audience, so a token signed with the shared secret could not
//! be kept apart.

use jsonwebtoken::{decode, decode_header, encode, Algorithm, Header, Validation};
use mairie360_api_lib::env_manager::get_env_var;
use mairie360_api_lib::jwt_manager::{
    session_public_keys, session_signing_key, SESSION_JWT_ISSUER,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Lifetime of a link token: the administrator follows the link right after asking for it.
pub const LINK_TOKEN_TTL_SECONDS: u64 = 120;

/// Lifetime of the console cookie: one administration session, then a new link is needed.
pub const CONSOLE_COOKIE_TTL_SECONDS: u64 = 30 * 60;

/// `aud` of the link tokens.
pub const LINK_TOKEN_AUDIENCE: &str = "keycloak-console";

/// `aud` of the console cookies.
pub const CONSOLE_COOKIE_AUDIENCE: &str = "keycloak-console-cookie";

/// `typ` header of the link tokens.
pub const LINK_TOKEN_TYPE: &str = "kc-console-link+jwt";

/// `typ` header of the console cookies.
pub const CONSOLE_COOKIE_TYPE: &str = "kc-console-cookie+jwt";

/// Name of the console cookie. The `__Secure-` prefix makes browsers refuse it unless it is
/// `Secure` and set over HTTPS.
pub const CONSOLE_COOKIE_NAME: &str = "__Secure-mairie360-kc-console";

/// Path of the console cookie: the browser only sends it to the Keycloak admin paths.
pub const CONSOLE_COOKIE_PATH: &str = "/admin";

/// Path of the public route that turns a link token into a console cookie.
pub const ADMIN_ACCESS_PATH: &str = "/admin-access";

/// Path of the route the `ForwardAuth` middleware calls, in the cluster only.
pub const VERIFY_PATH: &str = "/internal/keycloak-console/verify";

/// Redis key (prefixed with the role by the lib) recording a used link token until it expires.
#[must_use]
pub fn used_link_key(jti: &str) -> String {
    format!("keycloak-console:used-link:{jti}")
}

/// Where the console links point: the public Keycloak host and the instance realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleAccessConfig {
    public_url: String,
    realm: String,
}

impl ConsoleAccessConfig {
    /// Builds the configuration from a realm URL `<public host>/realms/<realm>`, normally the
    /// public issuer (`https://auth.dev.mairie360-eip.fr/realms/mairie360`).
    ///
    /// `None` when the URL has no `/realms/<realm>` segment or the realm name holds anything but
    /// letters, digits, `-` and `_` (it is written into a redirect path).
    #[must_use]
    pub fn from_realm_url(realm_url: &str) -> Option<Self> {
        const REALMS: &str = "/realms/";
        let realm_url = realm_url.trim().trim_end_matches('/');
        let index = realm_url.rfind(REALMS)?;
        let public_url = &realm_url[..index];
        let realm = &realm_url[index + REALMS.len()..];
        let valid_realm = !realm.is_empty()
            && realm
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        let valid_url = public_url.starts_with("https://") || public_url.starts_with("http://");
        (valid_realm && valid_url).then(|| Self {
            public_url: public_url.to_string(),
            realm: realm.to_string(),
        })
    }

    /// Reads `KEYCLOAK_ISSUER` (the public realm URL, set by the chart), else
    /// `KEYCLOAK_REALM_URL`. `None` disables console access: the routes answer `503`.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let realm_url = get_env_var("KEYCLOAK_ISSUER")
            .filter(|value| !value.trim().is_empty())
            .or_else(|| get_env_var("KEYCLOAK_REALM_URL").filter(|v| !v.trim().is_empty()))?;
        let config = Self::from_realm_url(&realm_url);
        if config.is_none() {
            tracing::warn!(
                "Keycloak console access disabled: KEYCLOAK_ISSUER / KEYCLOAK_REALM_URL is not a <url>/realms/<realm> URL."
            );
        }
        config
    }

    /// Public URL of the Keycloak host, without trailing slash.
    #[must_use]
    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    #[must_use]
    pub fn realm(&self) -> &str {
        &self.realm
    }

    /// The link handed to the administrator. A JWT only holds URL-safe characters.
    #[must_use]
    pub fn access_link(&self, link_token: &str) -> String {
        format!("{}{ADMIN_ACCESS_PATH}?token={link_token}", self.public_url)
    }

    /// Path of the realm admin console, where `GET /admin-access` redirects.
    #[must_use]
    pub fn console_path(&self) -> String {
        format!("/admin/{}/console/", self.realm)
    }
}

/// Claims of the link tokens and the console cookies.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsoleClaims {
    sub: String,
    iss: String,
    aud: String,
    iat: u64,
    exp: u64,
    jti: String,
}

impl ConsoleClaims {
    /// Id of the administrator the token was issued to.
    #[must_use]
    pub fn user_id(&self) -> Option<u64> {
        self.sub.parse().ok()
    }

    /// Unique id of the token.
    #[must_use]
    pub fn jti(&self) -> &str {
        &self.jti
    }

    /// Seconds left before the token expires (0 once expired).
    #[must_use]
    pub fn remaining_lifetime(&self) -> u64 {
        self.exp.saturating_sub(now())
    }
}

/// The two kinds of console tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleTokenKind {
    /// Single-use token of the link, read by `GET /admin-access`.
    Link,
    /// Console cookie, read by the verify route.
    Cookie,
}

impl ConsoleTokenKind {
    const fn audience(self) -> &'static str {
        match self {
            Self::Link => LINK_TOKEN_AUDIENCE,
            Self::Cookie => CONSOLE_COOKIE_AUDIENCE,
        }
    }

    const fn token_type(self) -> &'static str {
        match self {
            Self::Link => LINK_TOKEN_TYPE,
            Self::Cookie => CONSOLE_COOKIE_TYPE,
        }
    }

    /// Lifetime of a token of this kind.
    #[must_use]
    pub const fn ttl_seconds(self) -> u64 {
        match self {
            Self::Link => LINK_TOKEN_TTL_SECONDS,
            Self::Cookie => CONSOLE_COOKIE_TTL_SECONDS,
        }
    }
}

/// Why a console token could not be issued or was refused. The variants are logged, never sent
/// to the client, which gets one message for every refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleTokenError {
    /// `JWT_PRIVATE_KEY` / `JWT_PUBLIC_KEY` are not configured (or invalid): no token can be
    /// issued nor verified.
    NotConfigured,
    /// The token could not be signed.
    Signing,
    /// Not a JWT, or its header cannot be read.
    Malformed,
    /// Not an `EdDSA` token of the expected `typ` (a session JWT, the other console token...).
    WrongType,
    /// Its `kid` is not one of Core's keys.
    UnknownKey,
    /// Bad signature, issuer or audience, or a claim is missing.
    Invalid,
    /// Expired.
    Expired,
}

impl std::fmt::Display for ConsoleTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            Self::NotConfigured => "signing keys not configured",
            Self::Signing => "signing failed",
            Self::Malformed => "malformed token",
            Self::WrongType => "wrong token type",
            Self::UnknownKey => "unknown key id",
            Self::Invalid => "invalid signature or claims",
            Self::Expired => "expired",
        };
        f.write_str(reason)
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Issues a console token of `kind` for `user_id`, signed with Core's Ed25519 session key.
/// Returns the token and its `jti`.
///
/// # Errors
///
/// [`ConsoleTokenError::NotConfigured`] without a valid `JWT_PRIVATE_KEY` published in
/// `JWT_PUBLIC_KEY`, [`ConsoleTokenError::Signing`] when signing fails.
pub fn issue(kind: ConsoleTokenKind, user_id: u64) -> Result<(String, String), ConsoleTokenError> {
    let key = session_signing_key()
        .ok()
        .flatten()
        .ok_or(ConsoleTokenError::NotConfigured)?;
    let issued_at = now();
    let claims = ConsoleClaims {
        sub: user_id.to_string(),
        iss: SESSION_JWT_ISSUER.to_string(),
        aud: kind.audience().to_string(),
        iat: issued_at,
        exp: issued_at.saturating_add(kind.ttl_seconds()),
        jti: Uuid::new_v4().to_string(),
    };
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(key.kid().to_string());
    header.typ = Some(kind.token_type().to_string());
    let token =
        encode(&header, &claims, key.encoding_key()).map_err(|_| ConsoleTokenError::Signing)?;
    Ok((token, claims.jti))
}

/// Verifies a console token of `kind`: `EdDSA` with its own `typ`, a `kid` of `JWT_PUBLIC_KEY`,
/// the signature, `iss` = `mairie360-core`, its own `aud`, expiry (no leeway) and a `jti`.
///
/// # Errors
///
/// See [`ConsoleTokenError`].
pub fn verify(kind: ConsoleTokenKind, token: &str) -> Result<ConsoleClaims, ConsoleTokenError> {
    let header = decode_header(token).map_err(|_| ConsoleTokenError::Malformed)?;
    if header.alg != Algorithm::EdDSA || header.typ.as_deref() != Some(kind.token_type()) {
        return Err(ConsoleTokenError::WrongType);
    }
    let keys = session_public_keys().map_err(|_| ConsoleTokenError::NotConfigured)?;
    if keys.is_empty() {
        return Err(ConsoleTokenError::NotConfigured);
    }
    let key = header
        .kid
        .as_deref()
        .and_then(|kid| keys.iter().find(|key| key.kid() == kid))
        .ok_or(ConsoleTokenError::UnknownKey)?;
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.leeway = 0;
    validation.set_issuer(&[SESSION_JWT_ISSUER]);
    validation.set_audience(&[kind.audience()]);
    validation.set_required_spec_claims(&["exp", "sub", "iss", "aud"]);
    let claims = decode::<ConsoleClaims>(token, key.decoding_key(), &validation)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ConsoleTokenError::Expired,
            _ => ConsoleTokenError::Invalid,
        })?
        .claims;
    if claims.jti.is_empty() || claims.user_id().is_none() {
        return Err(ConsoleTokenError::Invalid);
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_read_from_the_public_issuer() {
        let config = ConsoleAccessConfig::from_realm_url(
            "https://auth.dev.mairie360-eip.fr/realms/mairie360/",
        )
        .unwrap();
        assert_eq!(config.public_url(), "https://auth.dev.mairie360-eip.fr");
        assert_eq!(config.realm(), "mairie360");
        assert_eq!(
            config.access_link("a.b.c"),
            "https://auth.dev.mairie360-eip.fr/admin-access?token=a.b.c"
        );
        assert_eq!(config.console_path(), "/admin/mairie360/console/");
    }

    #[test]
    fn config_refuses_urls_without_a_safe_realm() {
        assert!(ConsoleAccessConfig::from_realm_url("https://auth.example.org").is_none());
        assert!(ConsoleAccessConfig::from_realm_url("https://auth.example.org/realms/").is_none());
        assert!(
            ConsoleAccessConfig::from_realm_url("https://auth.example.org/realms/a%2Fb").is_none()
        );
        assert!(ConsoleAccessConfig::from_realm_url("javascript:x/realms/town").is_none());
    }

    #[test]
    fn malformed_tokens_are_refused_before_any_key_lookup() {
        assert_eq!(
            verify(ConsoleTokenKind::Link, "not-a-jwt").unwrap_err(),
            ConsoleTokenError::Malformed
        );
    }

    #[test]
    fn used_link_keys_are_namespaced() {
        assert_eq!(
            used_link_key("42"),
            "keycloak-console:used-link:42".to_string()
        );
    }
}
