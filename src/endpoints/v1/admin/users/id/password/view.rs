use crate::endpoints::validation::{check_password, Validate, ValidationError};
use serde::Deserialize;
use utoipa::ToSchema;

pub use crate::endpoints::validation::MIN_PASSWORD_LENGTH;
pub const MAX_PASSWORD_LENGTH: usize = 255;

/// New password set by an administrator.
#[derive(Deserialize, ToSchema)]
pub struct AdminResetPasswordView {
    /// New password, 12 to 255 characters (counted in characters, not in bytes), not a commonly used password, not containing the account's first name or last name.
    #[schema(format = Password, min_length = 12, max_length = 255, example = "NouveauMotDePasse!123")]
    new_password: String,
}

impl AdminResetPasswordView {
    pub fn new_password(&self) -> &str {
        &self.new_password
    }
}

impl Validate for AdminResetPasswordView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_password("new_password", &self.new_password, MIN_PASSWORD_LENGTH)
    }
}
