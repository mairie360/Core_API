use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginView {
    email: String,
    password: String,
    device_info: String,
}

impl LoginView {
    pub fn email(&self) -> String {
        self.email.clone()
    }

    pub fn password(&self) -> String {
        self.password.clone()
    }

    pub fn device_info(&self) -> String {
        self.device_info.clone()
    }
}

impl Display for LoginView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginView {{ email: {}, password: {}, device_info: {} }}",
            self.email, self.password, self.device_info
        )
    }
}
