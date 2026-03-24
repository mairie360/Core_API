use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct RegisterView {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    phone_number: Option<String>,
}

impl RegisterView {
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
}

impl Display for RegisterView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RegisterView {{ first_name: {}, last_name: {}, email: {}, password: {}, phone_number: {:?} }}",
            self.first_name, self.last_name, self.email, self.password, self.phone_number
        )
    }
}
