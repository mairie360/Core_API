use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct DoesRoleExistQueryView {
    id: u64,
}

impl DoesRoleExistQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}

impl DatabaseQueryView for DoesRoleExistQueryView {
    fn get_request(&self) -> String {
        format!("SELECT EXISTS(SELECT 1 FROM roles WHERE id = {})", self.id)
    }
}

impl Display for DoesRoleExistQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesRoleExistQueryView: id = {}", self.id)
    }
}
