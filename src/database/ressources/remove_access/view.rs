use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct RemoveAccessQueryView {
    id: u64,
}

impl RemoveAccessQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl DatabaseQueryView for RemoveAccessQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM access_control WHERE id = $1".to_string()
    }
}

impl Display for RemoveAccessQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteAccess: id = {}", self.id)
    }
}
