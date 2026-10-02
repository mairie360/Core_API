use crate::database::ids::id_to_sql;
use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

/// Removes `user_id` from `group_id`. Run with `fetch_scalar`, it returns the number of
/// memberships removed (`i64`, 0 or 1): the check and the removal are one statement (MAIR-420).
#[derive(serde::Deserialize)]
pub struct DeleteUserFromGroupQueryView {
    group_id: u64,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl DeleteUserFromGroupQueryView {
    #[must_use]
    pub fn new(group_id: u64, user_id: u64) -> Self {
        Self {
            group_id,
            user_id,
            params: vec![
                QueryParam::I32(id_to_sql(group_id)),
                QueryParam::I32(id_to_sql(user_id)),
            ],
        }
    }

    #[must_use]
    pub const fn group_id(&self) -> u64 {
        self.group_id
    }

    #[must_use]
    pub const fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for DeleteUserFromGroupQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH removed AS (\
            DELETE FROM group_members WHERE group_id = $1 AND user_id = $2 RETURNING user_id\
        ) SELECT COUNT(*) FROM removed"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DeleteUserFromGroupQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeleteUserFromGroupQueryView: group_id = {}, user_id = {}",
            self.group_id, self.user_id
        )
    }
}
