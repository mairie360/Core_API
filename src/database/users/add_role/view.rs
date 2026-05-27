use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct AddRolesQueryView {
    role_id: u64,
    user_id: u64,
}

impl AddRolesQueryView {
    pub fn new(role_id: u64, user_id: u64) -> Self {
        Self { role_id, user_id }
    }

    pub fn role_id(&self) -> u64 {
        self.role_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for AddRolesQueryView {
    fn get_request(&self) -> String {
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)".to_string()
    }
}

impl Display for AddRolesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AddRolesQueryView: role_id = {}, user_id = {}",
            self.role_id, self.user_id
        )
    }
}
