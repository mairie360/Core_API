use crate::endpoints::validation::{check_email, Validate, ValidationError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Demande d'e-mail de réinitialisation du mot de passe.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForgotPasswordView {
    /// Adresse e-mail du compte dont le mot de passe doit être réinitialisé.
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
