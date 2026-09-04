use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

#[derive(serde::Deserialize)]
pub struct DeleteUserFromGroupQueryView {
    group_id: u64,
    user_id: u64,
    params: Vec<QueryParam>,
}

impl DeleteUserFromGroupQueryView {
    pub fn new(group_id: u64, user_id: u64) -> Self {
        Self {
            group_id,
            user_id,
            params: vec![
                QueryParam::I32(group_id as i32),
                QueryParam::I32(user_id as i32),
            ],
        }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for DeleteUserFromGroupQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2"
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
