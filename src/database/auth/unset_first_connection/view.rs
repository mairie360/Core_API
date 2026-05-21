use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct UnsetFirstConnectionQueryView {
    user_id: u64,
    password: String,
}

impl UnsetFirstConnectionQueryView {
    pub fn new(user_id: u64, password: &str) -> Self {
        Self {
            user_id,
            password: password.to_string(),
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

impl DatabaseQueryView for UnsetFirstConnectionQueryView {
    fn get_request(&self) -> String {
        "UPDATE users SET first_connect = false AND password = $1 WHERE id = $2".to_string()
    }
}

impl Display for UnsetFirstConnectionQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UnsetFirstConnectionQueryView: user_id = {}, password = [PROTECTED]",
            self.user_id
        )
    }
}
