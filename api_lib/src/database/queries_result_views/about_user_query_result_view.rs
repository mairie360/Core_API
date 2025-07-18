use super::QueryResult;
use crate::database::db_interface::QueryResultView;

pub struct AboutUserQueryResultView {
}

impl AboutUserQueryResultView {
    pub fn new() -> Self {
        Self { }
    }
}

impl QueryResultView for AboutUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Boolean(true)
    }
}
