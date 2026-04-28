use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetActiveSessionsQueryView {
    user_id: u64,
}

impl GetActiveSessionsQueryView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for GetActiveSessionsQueryView {
    fn get_request(&self) -> String {
        "SELECT * FROM v_sessions WHERE user_id = $1 AND is_active = true".to_string()
    }
}

impl Display for GetActiveSessionsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetActiveSessionsQueryView: user_id = {}", self.user_id,)
    }
}
