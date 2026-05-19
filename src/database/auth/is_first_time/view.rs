use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct IsFirstTimeQueryView {
    user_id: u64,
}

impl IsFirstTimeQueryView {
    pub fn new(user_id: u64) -> Self {
        Self { user_id }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for IsFirstTimeQueryView {
    fn get_request(&self) -> String {
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) AS first_connect".to_string()
    }
}

impl Display for IsFirstTimeQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsFirstTimeQueryView: user_id = {}", self.user_id)
    }
}
