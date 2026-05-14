use std::fmt::Display;

use mairie360_api_lib::database::db_interface::DatabaseQueryView;

pub struct DeleteGroupQueryView {
    group_id: u64,
}

impl DeleteGroupQueryView {
    pub fn new(group_id: u64) -> Self {
        Self { group_id }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl DatabaseQueryView for DeleteGroupQueryView {
    fn get_request(&self) -> String {
        "DELETE FROM groups WHERE id = $1".to_string()
    }
}

impl Display for DeleteGroupQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteGroupQueryView: group_id = {}", self.group_id)
    }
}
