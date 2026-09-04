use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetUserRolesQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl GetUserRolesQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id as i32)],
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl ApiRequestDto for GetUserRolesQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(role_id) FROM user_roles WHERE user_id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserRolesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserRolesQueryView: id = {}", self.id)
    }
}
