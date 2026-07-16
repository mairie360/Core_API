use mairie360_api_lib::database::db_interface::DatabaseQueryView;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub struct GetUserByIdQueryView {
    id: u64,
}

impl GetUserByIdQueryView {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }
}

impl DatabaseQueryView for GetUserByIdQueryView {
    fn get_request(&self) -> String {
        "SELECT first_name, last_name, email, phone_number, status, is_archived FROM users WHERE id = $1"
            .to_string()
    }
}

impl Display for GetUserByIdQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetUserByIdQueryView: id = {}", self.id)
    }
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
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
