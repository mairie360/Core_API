use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct DeleteRoleQueryView {
    id: u64,
}

impl DeleteRoleQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl DatabaseQueryView for DeleteRoleQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM roles WHERE id = $1".to_string()
    }
}

impl Display for DeleteRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteRoleQueryView: id = {}", self.id,)
    }
}
