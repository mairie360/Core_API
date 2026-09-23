use crate::endpoints::validation::{
    check_password, Validate, ValidationError, MIN_PASSWORD_LENGTH,
};
use serde::Deserialize;
use utoipa::ToSchema;

/// Nouveau mot de passe imposé par un administrateur.
#[derive(Deserialize, ToSchema)]
pub struct AdminResetPasswordView {
    /// Nouveau mot de passe, de 8 à 255 caractères (comptés en caractères, pas en octets).
    #[schema(format = Password, min_length = 8, max_length = 255, example = "NouveauMotDePasse!123")]
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
