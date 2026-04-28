use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct GetSessionByTokenQueryView {
    token: String,
}

impl GetSessionByTokenQueryView {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn get_token(&self) -> &str {
        &self.token
    }
}

impl DatabaseQueryView for GetSessionByTokenQueryView {
    fn get_request(&self) -> String {
        "SELECT * FROM sessions WHERE token_hash = $1".to_string()
    }
}

impl Display for GetSessionByTokenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetSessionByTokenQueryView: token = {}", self.token)
    }
}
