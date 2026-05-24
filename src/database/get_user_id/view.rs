use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetUserIdQueryView {
    email: String,
}

impl GetUserIdQueryView {
    pub fn new(email: &str) -> Self {
        Self {
            email: email.to_string(),
        }
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

impl DatabaseQueryView for GetUserIdQueryView {
    fn get_request(&self) -> String {
        "SELECT id FROM users WHERE email = $1".to_string()
    }
}

impl Display for GetUserIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserIdQueryView: email = {}", self.email)
    }
}
