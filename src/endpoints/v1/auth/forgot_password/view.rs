use crate::endpoints::validation::{check_email, Validate, ValidationError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request for a password reset e-mail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForgotPasswordView {
    /// E-mail address of the account whose password must be reset.
    #[schema(max_length = 320, format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
}

impl ForgotPasswordView {
    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }
}

impl Validate for ForgotPasswordView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_email("email", &self.email)
    }
}
