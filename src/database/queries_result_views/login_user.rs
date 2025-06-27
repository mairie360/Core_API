use super::QueryResult;
use crate::database::db_interface::QueryResultView;

pub struct LoginUserQueryResultView {
    nb: u64,
}

impl LoginUserQueryResultView {
    pub fn new(nb: u64) -> Self {
        Self { nb }
    }
}

impl QueryResultView for LoginUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::U64(self.nb)
    }
}
