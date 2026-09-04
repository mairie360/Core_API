use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct RemoveAccessQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl RemoveAccessQueryView {
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

impl ApiRequestDto for RemoveAccessQueryView {
    fn query_sql(&self) -> &'static str {
        "DELETE FROM access_control WHERE id = $1"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for RemoveAccessQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeleteAccess: id = {}", self.id)
    }
}
