use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetSessionByTokenQueryView {
    token: String,
    params: Vec<QueryParam>,
}

impl GetSessionByTokenQueryView {
    pub fn new(token: String) -> Self {
        Self {
            params: vec![QueryParam::Text(token.clone())],
            token,
        }
    }

    pub fn get_token(&self) -> &str {
        &self.token
    }
}

impl ApiRequestDto for GetSessionByTokenQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT * FROM sessions WHERE token_hash = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetSessionByTokenQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetSessionByTokenQueryView: token = {}", self.token)
    }
}
