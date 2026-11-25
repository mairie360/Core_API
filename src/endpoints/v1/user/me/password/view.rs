use crate::endpoints::validation::{
    check_password, Validate, ValidationError, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Body of `POST /api/v1/user/me/password`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordView {
    /// Password the account signs in with today.
    #[schema(max_length = 255, example = "Old-password-42")]
    current_password: String,
    /// New password: 12 to 255 characters, no control character, not a commonly used password,
    /// different from the current one.
    #[schema(min_length = 12, max_length = 255, example = "New-password-2026")]
    new_password: String,
}

impl ChangePasswordView {
    #[must_use]
    pub fn current_password(&self) -> &str {
        &self.current_password
    }

    #[must_use]
    pub fn new_password(&self) -> &str {
        &self.new_password
    }
}

impl Validate for ChangePasswordView {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.current_password.chars().count() > MAX_PASSWORD_LENGTH {
            return Err(ValidationError::new(
                "current_password",
                &format!("must be at most {MAX_PASSWORD_LENGTH} characters"),
            ));
        }
        check_password("new_password", &self.new_password, MIN_PASSWORD_LENGTH)
    }
}
