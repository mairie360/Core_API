//! JWTs bound to a session (MAIR-226).
//!
//! The lib's `generate_jwt` only carries the user id (`sub`), so Core could not tell which
//! session a JWT belongs to. Core now issues its JWTs with an extra `sid` claim: the id of the
//! `sessions` row created at login (and reused on refresh). The lib ignores unknown claims, so
//! every API still accepts these JWTs; Core itself uses `sid` to revoke the current session on
//! logout and to reject the JWT of a revoked session (see `endpoints::session_guard`).
//!
//! Since MAIR-529 the JWT also carries the caller's role in `role`, so the BFFs no longer ask
//! `BFF_user /me` for it: a user has one role (enforced by the database since MAIR-564), the same
//! as `role` of `GET /user/me/`. It is read from the database at login and at refresh: a role
//! change shows in the JWT at the next refresh only.
//!
//! Since MAIR-680 signing and verification are the lib's (`mairie360_api_lib::jwt_manager`): Core
//! signs with its Ed25519 key (`JWT_PRIVATE_KEY`, `alg` `EdDSA`, `kid`, `iss`, `aud`) when it is
//! configured, else with `JWT_SECRET` (HS256, transition). The other services only hold the public
//! key (`JWT_PUBLIC_KEY`): they can verify a session JWT, never issue one.

use crate::database::roles::get_roles_by_id::Role;
use crate::database::roles::get_roles_of_user::GetRolesOfUserQueryView;
use jsonwebtoken::errors::{Error, ErrorKind};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::jwt_manager::{decode_jwt, generate_session_jwt as sign_session_jwt};
use mairie360_api_lib::state::AppState;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// The claims of a verified session JWT Core reads (the lib's `Claims`, with a typed session id).
#[derive(Debug)]
pub struct SessionClaims {
    sub: String,
    /// Name of the user's role, empty string when the user has none (and in JWTs issued before
    /// MAIR-529).
    role: String,
    exp: usize,
    /// Id of the `sessions` row. Absent from JWTs issued before MAIR-226.
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

    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
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

/// Name of the role `user_id` holds (one per user since MAIR-564; the lowest role id on a database
/// older than that), or an empty string when it holds none: the `role` claim of its JWTs.
///
/// # Errors
///
/// Returns the database error when the roles cannot be read.
pub async fn user_role_name(state: &AppState, user_id: u64) -> Result<String, ApiLibError> {
    let roles: Vec<Role> = state
        .get_smart_db()
        .fetch_all(&GetRolesOfUserQueryView::new(user_id))
        .await?;
    Ok(roles
        .first()
        .map(|role| role.name().to_string())
        .unwrap_or_default())
}

/// Issues a JWT for `user_id`, bound to the session `session_id` and carrying `role` (see
/// [`user_role_name`]), signed by the lib (Ed25519 key when configured, else `JWT_SECRET`).
///
/// # Errors
///
/// Returns an error when no signing key is configured, `JWT_TIMEOUT` is missing, or the token
/// cannot be signed.
pub fn generate_session_jwt(user_id: u64, session_id: Uuid, role: String) -> Result<String, Error> {
    sign_session_jwt(&user_id.to_string(), &role, Some(&session_id.to_string()))
}

/// Decodes and verifies `jwt` like every API does (`mairie360_api_lib::jwt_manager::decode_jwt`:
/// signature, `iss`/`aud` of the Ed25519 tokens, expiry without leeway).
///
/// # Errors
///
/// Returns an error when the token is invalid or expired, its session id is not a UUID, or no
/// verification key is configured.
pub fn decode_session_jwt(jwt: &str) -> Result<SessionClaims, Error> {
    let claims = decode_jwt(jwt)?;
    let sid = claims
        .session_id()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| Error::from(ErrorKind::InvalidToken))?;
    Ok(SessionClaims {
        sub: claims.user_id().to_string(),
        role: claims.role().to_string(),
        exp: claims.expiration(),
        sid,
    })
}
