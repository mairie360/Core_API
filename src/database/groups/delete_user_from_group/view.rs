use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

pub struct DeleteUserFromGroupQueryView {
    group_id: u64,
    user_id: u64,
}

impl DeleteUserFromGroupQueryView {
    pub fn new(group_id: u64, user_id: u64) -> Self {
        Self {
            group_id,
            user_id,
        }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for DeleteUserFromGroupQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM group_users WHERE group_id = $1 AND user_id = $2".to_string()
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
