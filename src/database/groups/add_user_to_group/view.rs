use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

pub struct AddUserToGroupQueryView {
    group_id: u64,
    user_id: u64,
}

impl AddUserToGroupQueryView {
    pub fn new(group_id: u64, user_id: u64) -> Self {
        Self { group_id, user_id }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for AddUserToGroupQueryView {
    fn get_request(&self) -> String {
        "INSERT INTO group_users (group_id, user_id) VALUES ($1, $2)".to_string()
    }
}

impl Display for AddUserToGroupQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddUserToGroupQueryView: group_id = {}, user_id = {}",
            self.group_id, self.user_id
        )
    }
}
