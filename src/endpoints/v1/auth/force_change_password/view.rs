use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForceChangePasswordView {
    token: String,
    new_password: String,
}

impl ForceChangePasswordView {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn new_password(&self) -> &str {
        &self.new_password
    }
}
