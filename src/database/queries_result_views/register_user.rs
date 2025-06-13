use crate::database::db_interface::QueryResultView;
use super::QueryResult;

pub struct RegisterUserQueryResultView {
    success: Result<(), String>,
}

impl RegisterUserQueryResultView {
    pub fn new(success: Result<(), String>) -> Self {
        Self { success }
    }
}

impl QueryResultView for RegisterUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Result(self.success.clone())
    }
}