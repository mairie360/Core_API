use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct LoginUserQueryView {
    email: String,
    password: String,
    params: Vec<QueryParam>,
}

impl LoginUserQueryView {
    #[must_use]
    pub fn new(email: String, password: String) -> Self {
        Self {
            params: vec![QueryParam::Text(email.clone())],
            email,
            password,
        }
    }

    #[must_use]
    pub const fn get_email(&self) -> &String {
        &self.email
    }

    #[must_use]
    pub const fn get_password(&self) -> &String {
        &self.password
    }
}

impl ApiRequestDto for LoginUserQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT id, password, first_connect FROM users WHERE email = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
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

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LoginUserQueryResultView {
    #[serde(rename = "id")]
    user_id: i32,
    #[serde(rename = "password")]
    password: String,
    #[serde(rename = "first_connect")]
    first_connect: bool,
}

impl LoginUserQueryResultView {
    #[must_use]
    pub const fn new(user_id: i32, password: String, first_connect: bool) -> Self {
        Self {
            user_id,
            password,
            first_connect,
        }
    }

    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }

    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }

    #[must_use]
    pub const fn first_connect(&self) -> bool {
        self.first_connect
    }
}
