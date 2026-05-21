use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct ChangePasswordQueryView {
    password: String,
    user_id: u64,
}

impl ChangePasswordQueryView {
    pub fn new(password: &str, user_id: u64) -> Self {
        Self {
            password: password.to_string(),
            user_id,
        }
    }

    pub fn get_password(&self) -> &str {
        &self.password
    }

    pub fn get_user_id(&self) -> u64 {
        self.user_id
    }
}

impl DatabaseQueryView for ChangePasswordQueryView {
    fn get_request(&self) -> String {
        "UPDATE users SET password = $1 WHERE id = $2".to_string()
    }
}

impl Display for ChangePasswordQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChangePasswordQueryView: password = [PROTECTED]")
    }
}
