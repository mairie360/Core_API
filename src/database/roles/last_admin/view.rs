//! Queries of the last-administrator guard (MAIR-678).
//!
//! An instance must keep at least one active administrator: a user holding the role named
//! `Admin` (the name `is_admin()` checks, protected by `protect_role_names`) whose account is
//! not archived. Granting another role, revoking `Admin` and archiving an account all go through
//! one transaction that:
//!
//! 1. locks the `Admin` role row with [`LockAdminRoleQueryView`], so these writes run one after
//!    the other;
//! 2. writes;
//! 3. checks with [`HasActiveAdminQueryView`] that an active administrator is left, and rolls
//!    back otherwise.
//!
//! The check is a separate statement run after the lock: under `READ COMMITTED` each statement
//! takes a new snapshot, so it sees what the transaction that held the lock before committed.
use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Name of the administrator role, as `is_admin()` reads it.
const ADMIN_ROLE: &str = "Admin";

/// Locks the `Admin` role row until the end of the transaction.
///
/// `FOR NO KEY UPDATE` conflicts with itself and with the `FOR UPDATE` of the role endpoints
/// (`LockRoleQueryView`), so two guarded writes never overlap, but not with the `FOR KEY SHARE`
/// the foreign key of `user_roles` takes: granting the `Admin` role elsewhere is not blocked.
/// Returns the role id (`fetch_all::<i32>`, empty if the role does not exist).
#[derive(serde::Deserialize)]
pub struct LockAdminRoleQueryView {
    params: Vec<QueryParam>,
}

impl LockAdminRoleQueryView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: vec![QueryParam::Text(ADMIN_ROLE.to_string())],
        }
    }
}

impl Default for LockAdminRoleQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for LockAdminRoleQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(id) FROM roles WHERE name = $1 FOR NO KEY UPDATE"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for LockAdminRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LockAdminRoleQueryView")
    }
}

/// Whether at least one active administrator exists (`fetch_scalar::<bool>`).
#[derive(serde::Deserialize)]
pub struct HasActiveAdminQueryView {
    params: Vec<QueryParam>,
}

impl HasActiveAdminQueryView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: vec![QueryParam::Text(ADMIN_ROLE.to_string())],
        }
    }
}

impl Default for HasActiveAdminQueryView {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequestDto for HasActiveAdminQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM user_roles ur \
         JOIN roles r ON r.id = ur.role_id \
         JOIN users u ON u.id = ur.user_id \
         WHERE r.name = $1 AND u.is_archived IS NOT TRUE)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for HasActiveAdminQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HasActiveAdminQueryView")
    }
}

/// Whether `user_id` is an active administrator and no other active administrator exists
/// (`fetch_scalar::<bool>`).
///
/// Unlocked early check, used before Core mirrors a change into Keycloak, so that a refused
/// change does not touch the realm (disabling an account ends its Keycloak sessions). The
/// transaction of the write still makes the authoritative check.
#[derive(serde::Deserialize)]
pub struct IsLastActiveAdminQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl IsLastActiveAdminQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![
                QueryParam::I32(id_to_sql(user_id)),
                QueryParam::Text(ADMIN_ROLE.to_string()),
            ],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for IsLastActiveAdminQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT COALESCE(bool_and(ur.user_id = $1), false) FROM user_roles ur \
         JOIN roles r ON r.id = ur.role_id \
         JOIN users u ON u.id = ur.user_id \
         WHERE r.name = $2 AND u.is_archived IS NOT TRUE"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsLastActiveAdminQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsLastActiveAdminQueryView: user_id = {}", self.user_id)
    }
}
