use crate::database::db_interface::DatabaseQueryView;
use crate::database::QUERY;
use std::fmt::Display;

pub struct LoginUserQueryView {
    email: String,
    password: String,
    query: QUERY,
}

impl LoginUserQueryView {
    pub fn new(
        email: String,
        password: String,
    ) -> Self {
        Self {
            email,
            password,
            query: QUERY::LoginUser,
        }
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
        format!(
            "SELECT id FROM users WHERE email = '{}' AND password = '{}'",
            self.email, self.password
        )
    }

    fn get_query_type(&self) -> QUERY {
        self.query
    }
}

impl Display for LoginUserQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoginUserQueryView: email = {}, password = {}", self.email, self.password)
    }
}
