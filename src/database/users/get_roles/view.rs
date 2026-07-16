use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetUserRolesQueryView {
    id: u64,
}

impl GetUserRolesQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl DatabaseQueryView for GetUserRolesQueryView {
    fn get_request(&self) -> String {
        "SELECT role_id FROM user_roles WHERE user_id = $1".to_string()
    }
}

impl Display for GetUserRolesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserRolesQueryView: id = {}", self.id)
    }
}
