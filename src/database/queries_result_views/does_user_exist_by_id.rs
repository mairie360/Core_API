use super::QueryResult;
use crate::database::db_interface::QueryResultView;

pub struct DoesUserExistByIdQueryResultView {
    does_user_exist: bool,
}

impl DoesUserExistByIdQueryResultView {
    pub fn new(does_user_exist: bool) -> Self {
        Self { does_user_exist }
    }
}

impl QueryResultView for DoesUserExistByIdQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Boolean(self.does_user_exist)
    }
}
