//! JWTs bound to a session (MAIR-226).
//!
//! The lib's `generate_jwt` only carries the user id (`sub`), so Core could not tell which
//! session a JWT belongs to. Core now issues its JWTs with an extra `sid` claim: the id of the
//! `sessions` row created at login (and reused on refresh). The lib ignores unknown claims, so
//! every API still accepts these JWTs; Core itself uses `sid` to revoke the current session on
//! logout and to reject the JWT of a revoked session (see `endpoints::session_guard`).

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use mairie360_api_lib::jwt_manager::{get_jwt_secret, get_jwt_timeout};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Same claims as the lib's `Claims`, plus the session id.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionClaims {
    sub: String,
    role: String,
    exp: usize,
    /// Id of the `sessions` row. Absent from JWTs issued before MAIR-226.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sid: Option<Uuid>,
}

impl SessionClaims {
    #[must_use]
    pub fn user_id(&self) -> Option<u64> {
        self.sub.parse().ok()
    }

    #[must_use]
    pub const fn session_id(&self) -> Option<Uuid> {
        self.sid
    }

    /// Seconds left before the token expires (0 once expired).
    #[must_use]
    pub fn remaining_lifetime(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        u64::try_from(self.exp).map_or(0, |exp| exp.saturating_sub(now))
    }
}

/// Issues a JWT for `user_id`, bound to the session `session_id`.
///
/// # Errors
///
/// Returns an error when `JWT_SECRET` / `JWT_TIMEOUT` are missing or the token cannot be signed.
pub fn generate_session_jwt(
    user_id: u64,
    session_id: Uuid,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret()?;
    let timeout = get_jwt_timeout()?;
    // A clock before the epoch gives 0: the token is then already expired (fails safe).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let claims = SessionClaims {
        sub: user_id.to_string(),
        role: String::new(),
        exp: usize::try_from(now)
            .unwrap_or(usize::MAX)
            .saturating_add(timeout),
        sid: Some(session_id),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
}

/// Decodes and verifies `jwt` (signature and expiry, no leeway, like the lib).
///
/// # Errors
///
/// Returns an error when the token is invalid, expired, or `JWT_SECRET` is missing.
pub fn decode_session_jwt(jwt: &str) -> Result<SessionClaims, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret()?;
    let validation = Validation {
        leeway: 0,
        ..Default::default()
    };
    Ok(decode::<SessionClaims>(jwt, &DecodingKey::from_secret(&secret), &validation)?.claims)
}
