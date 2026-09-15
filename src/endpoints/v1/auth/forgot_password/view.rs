use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForgotPasswordView {
    email: String,
}

impl ForgotPasswordView {
    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }
}
