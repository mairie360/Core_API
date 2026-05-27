use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct DeleteUserQueryView {
    user_id: u64,
}

impl DeleteUserQueryView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for DeleteUserQueryView {
    fn get_request(&self) -> String {
        "SELECT delete_user($1)".to_string()
    }
}

impl Display for DeleteUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteUserQueryView: user_id = {}", self.user_id)
    }
}
