use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordView {
    /// Jeton de réinitialisation reçu par e-mail. À usage unique.
    #[schema(
        pattern = r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35"
    )]
    token: String,
    /// Nouveau mot de passe en clair.
    #[schema(format = Password, example = "NouveauMotDePasse!123")]
    new_password: String,
    /// Description libre de l'appareil, conservée sur la session ouverte par cet appel.
    #[schema(example = "Chrome 140 sur Windows 11")]
    device_info: String,
}

impl ResetPasswordView {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn new_password(&self) -> &str {
        &self.new_password
    }

    pub fn device_info(&self) -> String {
        self.device_info.clone()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ResetPasswordResponseView {
    /// Jeton opaque permettant d'obtenir un nouveau JWT via `POST /api/v1/sessions/refresh`.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    refresh_token: String,
}

impl ResetPasswordResponseView {
    pub fn new(refresh_token: String) -> Self {
        ResetPasswordResponseView { refresh_token }
    }

    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl Display for ResetPasswordResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResetPasswordResponseView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}

impl From<String> for ResetPasswordResponseView {
    fn from(token: String) -> Self {
        ResetPasswordResponseView {
            refresh_token: token,
        }
    }
}
