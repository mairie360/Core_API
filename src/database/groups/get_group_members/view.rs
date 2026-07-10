use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

pub struct GetGroupUsersQueryView {
    group_id: u64,
}

impl GetGroupUsersQueryView {
    pub fn new(group_id: u64) -> Self {
        Self { group_id }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl DatabaseQueryView for GetGroupUsersQueryView {
    fn get_request(&self) -> String {
        "SELECT user_id FROM group_members WHERE group_id = $1".to_string()
    }
}

impl Display for GetGroupUsersQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroupUsersQueryView: group_id = {}", self.group_id)
    }
}
