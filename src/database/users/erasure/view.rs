//! Erasure and export of a user (MAIR-289): `anonymize_user()` and `export_user_data()` of
//! Devops/Database (MAIR-289, `repeatable/users/anonymize_user.sql`), both `SECURITY DEFINER`
//! and granted to `core_api` only. They follow the `erasure` of every column in
//! `gdpr/inventory.yaml`: Core only calls them and mirrors the result (revocation list, Keycloak).
use crate::database::ids::id_to_sql;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

/// SQLSTATE of `anonymize_user()` / `export_user_data()` for an unknown user (`no_data_found`).
pub const UNKNOWN_USER: &str = "P0002";
/// SQLSTATE of `anonymize_user()` for the seeded administrator, or when no other active
/// administrator can take the user's groups, projects and events over (`restrict_violation`).
pub const ERASURE_REFUSED: &str = "23001";

/// `SELECT anonymize_user($1)`: erases the user (archived first if needed). Calling it again for
/// an anonymized account changes nothing and answers `already_anonymized`.
#[derive(Deserialize)]
pub struct AnonymizeUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl AnonymizeUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for AnonymizeUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT anonymize_user($1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for AnonymizeUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnonymizeUserQueryView: user_id = {}", self.user_id)
    }
}

/// What `anonymize_user()` did. `handed_over_to`, `deleted_private_events` and
/// `revoked_sessions` are absent when the account was already anonymized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnonymizeUserResult {
    pub user_id: i32,
    pub already_anonymized: bool,
    /// The administrator who took over the user's groups, projects and non-private events.
    #[serde(default)]
    pub handed_over_to: Option<i32>,
    /// Private events of the user, deleted.
    #[serde(default)]
    pub deleted_private_events: Option<i64>,
    /// Sessions of the user, deleted: Core publishes them to the revocation list.
    #[serde(default)]
    pub revoked_sessions: Vec<Uuid>,
}

/// `SELECT export_user_data($1)`: `{ user, data: { "<table>.<column>": [rows] }, exported_at }`,
/// every row the schema links to the user, credentials left out.
#[derive(Deserialize)]
pub struct ExportUserDataQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl ExportUserDataQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(id_to_sql(user_id))],
        }
    }
}

impl ApiRequestDto for ExportUserDataQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT export_user_data($1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for ExportUserDataQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExportUserDataQueryView: user_id = {}", self.user_id)
    }
}
