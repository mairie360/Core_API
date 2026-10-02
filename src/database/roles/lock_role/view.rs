use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Locks the role `id` until the end of the transaction and returns its `can_be_deleted` flag,
/// one JSON boolean (`fetch_all::<bool>`, empty when the role does not exist).
///
/// Run first in a transaction that checks the role then writes it (MAIR-420): a concurrent
/// write of the same role waits for the commit, so the checks still hold when the write runs.
#[derive(serde::Deserialize)]
pub struct LockRoleQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl LockRoleQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I64(id as i64)],
        }
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

impl ApiRequestDto for LockRoleQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(can_be_deleted) FROM roles WHERE id = $1 FOR UPDATE"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for LockRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LockRoleQueryView: id = {}", self.id)
    }
}
