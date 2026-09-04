use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct CanDeleteRoleQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl CanDeleteRoleQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I64(id as i64)],
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl ApiRequestDto for CanDeleteRoleQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT can_be_deleted FROM roles WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for CanDeleteRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CanDeleteRoleQueryView: id = {}", self.id,)
    }
}
