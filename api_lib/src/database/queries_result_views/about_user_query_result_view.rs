use super::QueryResult;
use crate::database::db_interface::QueryResultView;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
pub struct AboutUserQueryResultView {
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    status: String,
}

impl AboutUserQueryResultView {
    pub fn new(first_name: &str, last_name: &str, email: &str, phone: &str, status: &str) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.to_string(),
            status: status.to_string(),
        }
    }
}

impl QueryResultView for AboutUserQueryResultView {
    fn get_result(&self) -> QueryResult {
        let json = serde_json::to_value(&self).unwrap();
        QueryResult::JSON(json)
    }
}
