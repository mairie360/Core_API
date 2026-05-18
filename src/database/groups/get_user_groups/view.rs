use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

pub struct GetUserGroupsQuerView {
    user_id: u64,
}

impl GetUserGroupsQuerView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for GetUserGroupsQuerView {
    fn get_request(&self) -> String {
        "SELECT * FROM groups WHERE id = (Select group_id FROM group_users WHERE user_id = $1)"
            .to_string()
    }
}

impl Display for GetUserGroupsQuerView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetGroups: user_id = {}", self.user_id)
    }
}
