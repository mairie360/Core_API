use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct RemoveRolesQueryView {
    role_id: u64,
    user_id: u64,
}

impl RemoveRolesQueryView {
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

impl DatabaseQueryView for RemoveRolesQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM user_roles WHERE role_id = $1 AND user_id = $2".to_string()
    }
}

impl Display for RemoveRolesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RemoveRolesQueryView: role_id = {}, user_id = {}",
            self.role_id, self.user_id
        )
    }
}
