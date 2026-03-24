use serde::{Deserialize, Serialize};
use serde_json;

/**
 * AboutUserQueryResultView
 * This struct represents the result view for the "about user" query.
 * It contains the user's first name, last name, email, phone number, and status.
 * It implements the QueryResultView trait to provide a JSON representation of the data.
 */
#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct AboutUserQueryResultView {
    first_name: String,
    last_name: String,
    email: String,
    phone_number: String,
    status: String,
}

impl AboutUserQueryResultView {
    /**
     * Creates a new instance of AboutUserQueryResultView.
     *
     * # Arguments
     * * `first_name` - The first name of the user.
     * * `last_name` - The last name of the user.
     * * `email` - The email address of the user.
     * * `phone_number` - The phone number of the user.
     * * `status` - The status of the user (e.g., active, inactive).
     *
     * # Returns
     * A new instance of AboutUserQueryResultView.
     */
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
