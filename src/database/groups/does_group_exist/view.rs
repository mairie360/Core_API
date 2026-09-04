use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct DoesGroupExistQuerView {
    group_id: u64,
    params: Vec<QueryParam>,
}

impl DoesGroupExistQuerView {
    pub fn new(group_id: u64) -> Self {
        Self {
            group_id,
            params: vec![QueryParam::I32(group_id as i32)],
        }
    }

    pub fn group_id(&self) -> u64 {
        self.group_id
    }
}

impl ApiRequestDto for DoesGroupExistQuerView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM groups WHERE id = $1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for DoesGroupExistQuerView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DoesGroupExists: group_id = {}", self.group_id)
    }
}
