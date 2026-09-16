use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForgotPasswordView {
    /// Adresse e-mail du compte dont le mot de passe doit être réinitialisé.
    #[schema(format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
}

impl ForgotPasswordView {
    pub fn email(&self) -> &str {
        &self.email
    }
}
