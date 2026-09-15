use serde::Deserialize;
use utoipa::ToSchema;

pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 255;

#[derive(Deserialize, ToSchema)]
pub struct AdminResetPasswordView {
    new_password: String,
}

impl AdminResetPasswordView {
    pub fn new_password(&self) -> &str {
        &self.new_password
    }
}
