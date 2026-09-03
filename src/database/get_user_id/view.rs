use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetUserIdQueryView {
    email: String,
    params: Vec<QueryParam>,
}

impl GetUserIdQueryView {
    pub fn new(email: &str) -> Self {
        Self {
            email: email.to_string(),
            params: vec![QueryParam::Text(email.to_string())],
        }
    }

    pub fn email(&self) -> &str {
        &self.email
    }
}

impl ApiRequestDto for GetUserIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT id FROM users WHERE email = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserIdQueryView: email = {}", self.email)
    }
}
