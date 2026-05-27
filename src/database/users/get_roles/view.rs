use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetUserRolesdQueryView {
    id: u64,
}

impl GetUserRolesdQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl DatabaseQueryView for GetUserRolesdQueryView {
    fn get_request(&self) -> String {
        "SELECT role_id FROM user_roles WHERE user_id = $1".to_string()
    }
}

impl Display for GetUserRolesdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserRolesdQueryView: id = {}", self.id)
    }
}
