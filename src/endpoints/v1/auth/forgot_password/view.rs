use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Demande d'e-mail de réinitialisation du mot de passe.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForgotPasswordView {
    /// Adresse e-mail du compte dont le mot de passe doit être réinitialisé.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
}

impl ForgotPasswordView {
    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }
}
