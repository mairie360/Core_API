use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchMeView {
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

impl PatchMeView {
    pub fn new(
        first_name: Option<String>,
        last_name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email,
            phone,
        }
    }

    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }

    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }
}

impl Display for PatchMeView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchMeView {{ first_name: {:?}, last_name: {:?}, email: {:?}, phone: {:?} }}",
            self.first_name, self.last_name, self.email, self.phone
        )
    }
}
