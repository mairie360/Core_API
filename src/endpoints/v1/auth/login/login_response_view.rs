use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginResponseView {
    refresh_token: String,
}

impl LoginResponseView {
    pub fn new(refresh_token: String) -> Self {
        LoginResponseView { refresh_token }
    }

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
        LoginResponseView {
            refresh_token: token,
        }
    }
}
