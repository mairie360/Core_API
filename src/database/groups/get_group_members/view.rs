use std::fmt::Display;

use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};

#[derive(serde::Deserialize)]
pub struct GetGroupUsersQueryView {
    group_id: u64,
    params: Vec<QueryParam>,
}

impl GetGroupUsersQueryView {
    pub fn new(group_id: u64) -> Self {
        Self {
            group_id,
            params: vec![QueryParam::I32(group_id as i32)],
        }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl ApiRequestDto for GetGroupUsersQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(user_id) FROM group_members WHERE group_id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetGroupUsersQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroupUsersQueryView: group_id = {}", self.group_id)
    }
}
