use crate::endpoints::validation::{check_opaque, Validate, ValidationError, MAX_TOKEN_LENGTH};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Confirmation of a new e-mail address, from the token received at that address.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfirmEmailView {
    /// Token of the confirmation link e-mailed to the new address. Single-use, valid one hour.
    #[schema(
        pattern = r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35"
    )]
    token: String,
}

impl ConfirmEmailView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Validate for ConfirmEmailView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("token", &self.token, MAX_TOKEN_LENGTH)
    }
}
