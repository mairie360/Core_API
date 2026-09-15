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
    #[must_use]
    pub fn email(&self) -> String {
        self.email.clone()
    }

    #[must_use]
    pub fn password(&self) -> String {
        self.password.clone()
    }

    #[must_use]
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginResponseView {
    refresh_token: String,
}

impl LoginResponseView {
    #[must_use]
    pub const fn new(refresh_token: String) -> Self {
        Self { refresh_token }
    }

    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl Display for LoginResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginResponseView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}

impl From<String> for LoginResponseView {
    fn from(token: String) -> Self {
        Self {
            refresh_token: token,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginFirstConnectionResponseView {
    token: String,
}

impl LoginFirstConnectionResponseView {
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self { token }
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Display for LoginFirstConnectionResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ \"token\": {} }}", self.token)
    }
}

impl From<String> for LoginFirstConnectionResponseView {
    fn from(token: String) -> Self {
        Self { token }
    }
}
