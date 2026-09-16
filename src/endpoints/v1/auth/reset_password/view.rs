use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordView {
    token: String,
    new_password: String,
    device_info: String,
}

impl ResetPasswordView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn new_password(&self) -> &str {
        &self.new_password
    }

    #[must_use]
    pub fn device_info(&self) -> String {
        self.device_info.clone()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordResponseView {
    refresh_token: String,
}

impl ResetPasswordResponseView {
    #[must_use]
    pub const fn new(refresh_token: String) -> Self {
        Self { refresh_token }
    }

    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl Display for ResetPasswordResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResetPasswordResponseView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}

impl From<String> for ResetPasswordResponseView {
    fn from(token: String) -> Self {
        Self {
            refresh_token: token,
        }
    }
}
