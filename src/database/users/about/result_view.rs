use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct AboutUserQueryResultView {
    first_name: String,
    last_name: String,
    email: String,
    phone_number: String,
    status: String,
}

impl AboutUserQueryResultView {
    pub fn new(
        first_name: &str,
        last_name: &str,
        email: &str,
        phone_number: &str,
        status: &str,
    ) -> Self {
        Self {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone_number: phone_number.to_string(),
            status: status.to_string(),
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

    pub fn phone_number(&self) -> &str {
        &self.phone_number
    }

    pub fn status(&self) -> &str {
        &self.status
    }
}
