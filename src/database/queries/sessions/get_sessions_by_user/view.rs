use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetSessionsByUserQueryView {
    user_id: u64,
}

impl GetSessionsByUserQueryView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for GetSessionsByUserQueryView {
    fn get_request(&self) -> String {
        "SELECT * FROM sessions WHERE user_id = $1".to_string()
    }
}

impl Display for GetSessionsByUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetSessionsByUserQueryView: user_id = {}", self.user_id)
    }
}
