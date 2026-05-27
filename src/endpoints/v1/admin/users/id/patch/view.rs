use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct PatchUserView {
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone_number: Option<String>,
}

impl PatchUserView {
    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }

    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
}

impl Display for PatchUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchUserView {{ first_name: {}, last_name: {}, email: {}, phone_number: {:?} }}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or(""),
            self.email.as_deref().unwrap_or(""),
            self.phone_number.as_deref().unwrap_or("")
        )
    }
}
