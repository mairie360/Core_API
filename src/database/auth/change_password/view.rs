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
        "SELECT id, password, first_connect FROM users WHERE email = $1".to_string()
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

#[derive(Debug, sqlx::FromRow, PartialEq, Eq)]
pub struct LoginUserQueryResultView {
    #[sqlx(rename = "id")]
    user_id: i32,
    #[sqlx(rename = "password")]
    password: String,
    #[sqlx(rename = "first_connect")]
    first_connect: bool,
}

impl LoginUserQueryResultView {
    pub fn new(user_id: i32, password: String, first_connect: bool) -> Self {
        Self {
            user_id,
            password,
            first_connect,
        }
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn user_id(&self) -> i32 {
        self.user_id
    }

    pub fn first_connect(&self) -> bool {
        self.first_connect
    }
}
