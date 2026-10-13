//! JWTs bound to a session (MAIR-226).
//!
//! The lib's `generate_jwt` only carries the user id (`sub`), so Core could not tell which
//! session a JWT belongs to. Core now issues its JWTs with an extra `sid` claim: the id of the
//! `sessions` row created at login (and reused on refresh). The lib ignores unknown claims, so
//! every API still accepts these JWTs; Core itself uses `sid` to revoke the current session on
//! logout and to reject the JWT of a revoked session (see `endpoints::session_guard`).
//!
//! Since MAIR-529 the JWT also carries the caller's roles, so the BFFs no longer ask
//! `BFF_user /me` for them: `roles` holds every role name, `role` the first one (lowest role id,
//! like the deprecated `role` of `GET /user/me/`). Both are read from the database at login and
//! at refresh: a role change shows in the JWT at the next refresh only.

use crate::database::roles::get_roles_by_id::Role;
use crate::database::roles::get_roles_of_user::GetRolesOfUserQueryView;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::jwt_manager::{get_jwt_secret, get_jwt_timeout};
use mairie360_api_lib::state::AppState;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Same claims as the lib's `Claims`, plus the session id and the roles.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionClaims {
    sub: String,
    /// First of `roles`, empty string when the user has none (and in JWTs issued before MAIR-529).
    role: String,
    exp: usize,
    /// Id of the `sessions` row. Absent from JWTs issued before MAIR-226.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sid: Option<Uuid>,
    /// Names of every role the user holds, ordered by role id. Empty when the user has none;
    /// absent from JWTs issued before MAIR-529.
    #[serde(default)]
    roles: Vec<String>,
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

    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    #[must_use]
    pub fn roles(&self) -> &[String] {
        &self.roles
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

/// Names of the roles `user_id` holds, ordered by role id: the `roles` claim of its JWTs.
///
/// # Errors
///
/// Returns the database error when the roles cannot be read.
pub async fn user_role_names(state: &AppState, user_id: u64) -> Result<Vec<String>, ApiLibError> {
    let roles: Vec<Role> = state
        .get_smart_db()
        .fetch_all(&GetRolesOfUserQueryView::new(user_id))
        .await?;
    Ok(roles.iter().map(|role| role.name().to_string()).collect())
}

/// Issues a JWT for `user_id`, bound to the session `session_id` and carrying `roles` (see
/// [`user_role_names`]).
///
/// # Errors
///
/// Returns an error when `JWT_SECRET` / `JWT_TIMEOUT` are missing or the token cannot be signed.
pub fn generate_session_jwt(
    user_id: u64,
    session_id: Uuid,
    roles: Vec<String>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret()?;
    let timeout = get_jwt_timeout()?;
    // A clock before the epoch gives 0: the token is then already expired (fails safe).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let claims = SessionClaims {
        sub: user_id.to_string(),
        role: roles.first().cloned().unwrap_or_default(),
        exp: usize::try_from(now)
            .unwrap_or(usize::MAX)
            .saturating_add(timeout),
        sid: Some(session_id),
        roles,
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
