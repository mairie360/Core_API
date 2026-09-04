use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct IsFirstTimeQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl IsFirstTimeQueryView {
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }
}

impl ApiRequestDto for IsFirstTimeQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) AS first_connect"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for IsFirstTimeQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IsFirstTimeQueryView: user_id = {}", self.user_id)
    }
}
