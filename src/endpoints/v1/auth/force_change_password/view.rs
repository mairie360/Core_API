use crate::endpoints::validation::{
    check_opaque, check_password, Validate, ValidationError, MAX_TOKEN_LENGTH,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Changement de mot de passe imposé à la première connexion.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForceChangePasswordView {
    /// Jeton de première connexion renvoyé par le `412` de `POST /api/v1/auth/login`.
    #[schema(
        pattern = r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35"
    )]
    token: String,
    /// Mot de passe choisi par l'utilisateur, qui remplace celui attribué à la création du compte.
    #[schema(min_length = 1, max_length = 255, format = Password, example = "NouveauMotDePasse!123")]
    new_password: String,
}

impl ForceChangePasswordView {
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn new_password(&self) -> &str {
        &self.new_password
    }
}

impl Validate for ForceChangePasswordView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_opaque("token", &self.token, MAX_TOKEN_LENGTH)?;
        check_password("new_password", &self.new_password, 1)
    }
}
