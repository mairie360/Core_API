use super::QueryResult;
use crate::database::db_interface::QueryResultView;

pub struct LoginUserQueryResultView {
    nb: Result<u64, String>,
}

impl LoginUserQueryResultView {
    pub fn new(nb: Result<u64, String>) -> Self {
        Self { nb }
    }
}

impl QueryResultView for LoginUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        match &self.nb {
            Ok(nb) => QueryResult::U64(*nb),
            Err(e) => QueryResult::U64(0),
        }
    }
}
