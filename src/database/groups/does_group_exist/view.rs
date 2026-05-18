use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct DoesGroupExistQuerView {
    group_id: u64,
}

impl DoesGroupExistQuerView {
    pub fn new(group_id: u64) -> Self {
        Self { group_id }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl DatabaseQueryView for DoesGroupExistQuerView {
    fn get_request(&self) -> String {
        "SELECT EXISTS(SELECT 1 FROM groups WHERE id = $1)".to_string()
    }
}

impl Display for DoesGroupExistQuerView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesGroupExists: group_id = {}", self.group_id)
    }
}
