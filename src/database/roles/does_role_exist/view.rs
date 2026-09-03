use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct DoesRoleExistQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl DoesRoleExistQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I64(id as i64)],
        }
    }
}

impl ApiRequestDto for DoesRoleExistQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DoesRoleExistQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesRoleExistQueryView: id = {}", self.id)
    }
}
