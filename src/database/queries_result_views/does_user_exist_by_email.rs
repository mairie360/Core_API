use crate::database::db_interface::QueryResultView;
use super::QueryResult;

pub struct DoesUserExistByEmailQueryResultView {
    does_user_exist: bool,
}

impl DoesUserExistByEmailQueryResultView {
    pub fn new(does_user_exist: bool) -> Self {
        Self { does_user_exist }
    }
}

impl QueryResultView for DoesUserExistByEmailQueryResultView {
    fn get_result(&self) -> QueryResult {
        QueryResult::Boolean(self.does_user_exist)
    }
}