use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Archives a user through the schema's soft-delete view `v_users_active`.
///
/// Its `INSTEAD OF DELETE` trigger flags the row archived, which ends the user's sessions.
/// Users are never hard-deleted: `core_api` has no `DELETE` grant on `users`.
///
/// Deleting an unknown or already archived user matches no row and succeeds silently: check
/// the account first when that must be told apart. A user owning groups, events or projects
/// is refused by the schema (`restrict_violation`).
#[derive(serde::Deserialize)]
pub struct DeleteUserQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl DeleteUserQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for DeleteUserQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM v_users_active WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DeleteUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteUserQueryView: user_id = {}", self.user_id)
    }
}
