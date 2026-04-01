use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use std::fmt::Display;

pub struct LoginUserQueryView {
    email: String,
    password: String,
}

impl LoginUserQueryView {
    pub fn new(email: String, password: String) -> Self {
        Self { email, password }
    }

    pub fn get_email(&self) -> &String {
        &self.email
    }

    pub fn get_password(&self) -> &String {
        &self.password
    }
}

impl DatabaseQueryView for LoginUserQueryView {
    fn get_request(&self) -> String {
        "SELECT id, password FROM users WHERE email = $1".to_string()
    }
}

impl Display for LoginUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginUserQueryView: email = {}, password = [PROTECTED]",
            self.email
        )
    }
}
