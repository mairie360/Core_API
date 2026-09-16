use serde::Deserialize;
use utoipa::ToSchema;

pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 255;

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
