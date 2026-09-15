use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(serde::Deserialize)]
pub struct GetUserByIdQueryView {
    id: u64,
    params: Vec<QueryParam>,
}

impl GetUserByIdQueryView {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self {
            id,
            params: vec![QueryParam::I32(id as i32)],
        }
    }

    #[must_use]
    pub const fn get_id(&self) -> u64 {
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
    #[must_use]
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
            phone_number: phone_number.map(std::string::ToString::to_string),
            status: status.to_string(),
            is_archived,
        }
    }

    /// # Panics
    ///
    /// Panique si `Self` ne peut pas être sérialisé en JSON (ne devrait pas arriver : tous les
    /// champs sont des types `serde`-compatibles standards).
    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    #[must_use]
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    #[must_use]
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[must_use]
    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub const fn is_archived(&self) -> bool {
        self.is_archived
    }
}
