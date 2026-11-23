//! Suspension of an account by an administrator (MAIR-904).
//!
//! A suspended account (`users.status = 'suspended'`, Database `mair-904-01`) keeps every row
//! (roles, groups, factors, passkeys) but cannot sign in, its sessions are revoked, and
//! `is_admin()` / `check_access()` deny it. Reactivating gives it back `offline`, the status a
//! restored account gets.

use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Suspends a non-archived account, its row locked first.
///
/// Read with `fetch_scalar::<bool>`: `true` when the account was active and is now suspended, `false` when
/// it was already suspended. An unknown or archived account gives no row (`DbError::NotFound`).
#[derive(Debug, serde::Deserialize)]
pub struct SuspendUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl SuspendUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for SuspendUserQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH target AS (\
            SELECT id, status FROM users \
            WHERE id = $1 AND COALESCE(is_archived, false) = false FOR UPDATE\
        ), changed AS (\
            UPDATE users u SET status = 'suspended' FROM target t \
            WHERE u.id = t.id AND t.status IS DISTINCT FROM 'suspended' RETURNING u.id\
        ) SELECT EXISTS (SELECT 1 FROM changed) FROM target"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }

    fn invalidated_read_caches(&self) -> &'static [&'static str] {
        crate::database::users::list_directory::INVALIDATES_DIRECTORY
    }
}

impl Display for SuspendUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SuspendUserQueryView: user_id = {}", self.user_id)
    }
}

/// Reactivates a suspended, non-archived account (status `offline`).
///
/// Read with `fetch_scalar::<bool>`: `true` when it was suspended, `false` when it was not. An unknown or
/// archived account gives no row (`DbError::NotFound`).
#[derive(Debug, serde::Deserialize)]
pub struct ReactivateUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl ReactivateUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for ReactivateUserQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH target AS (\
            SELECT id, status FROM users \
            WHERE id = $1 AND COALESCE(is_archived, false) = false FOR UPDATE\
        ), changed AS (\
            UPDATE users u SET status = 'offline' FROM target t \
            WHERE u.id = t.id AND t.status = 'suspended' RETURNING u.id\
        ) SELECT EXISTS (SELECT 1 FROM changed) FROM target"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }

    fn invalidated_read_caches(&self) -> &'static [&'static str] {
        crate::database::users::list_directory::INVALIDATES_DIRECTORY
    }
}

impl Display for ReactivateUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReactivateUserQueryView: user_id = {}", self.user_id)
    }
}
