use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct DeleteRoleQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl DeleteRoleQueryView {
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

impl ApiRequestDto for DeleteRoleQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM roles WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DeleteRoleQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteRoleQueryView: id = {}", self.id,)
    }
}
