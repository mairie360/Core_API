use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetUserByIdQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl GetUserByIdQueryView {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id as i32)],
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl ApiRequestDto for GetUserByIdQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT row_to_json(t) FROM (SELECT first_name, last_name, email, phone_number, status, is_archived FROM users WHERE id = $1) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetUserByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserByIdQueryView: id = {}", self.id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetUserByIdQueryResultView {
    first_name: String,
    last_name: String,
    email: String,
    phone_number: Option<String>,
    status: String,
    is_archived: bool,
}

impl GetUserByIdQueryResultView {
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone_number: Option<&str>,
        status: &str,
        is_archived: bool,
    ) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone_number: phone_number.map(|p| p.to_string()),
            status: status.to_string(),
            is_archived,
        }
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn is_archived(&self) -> bool {
        self.is_archived
    }
}
