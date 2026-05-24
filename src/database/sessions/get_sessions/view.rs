use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetSessionsQueryView {
    id: Vec<i32>,
}

impl GetSessionsQueryView {
    pub fn new(id: Vec<u64>) -> Self {
        Self {
            id: id.into_iter().map(|id| id as i32).collect(),
        }
    }

    pub fn id(&self) -> &[i32] {
        &self.id
    }
}

impl DatabaseQueryView for GetSessionsQueryView {
    fn get_request(&self) -> String {
        "SELECT * FROM sessions WHERE user_id = ANY($1)".to_string()
    }
}

impl Display for GetSessionsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetSessionsQueryView: id = {:?}", self.id)
    }
}
