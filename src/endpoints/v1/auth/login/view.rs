use crate::endpoints::validation::{
    check_opaque, Validate, ValidationError, MAX_EMAIL_LENGTH, MAX_PASSWORD_LENGTH,
    MAX_TOKEN_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Identifiants de connexion.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginView {
    /// Adresse e-mail du compte.
    #[schema(max_length = 320, format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Mot de passe en clair, transmis tel quel sur le canal TLS.
    #[schema(max_length = 255, format = Password, example = "MotDePasse!123")]
    password: String,
    /// Description libre de l'appareil, conservée sur la session pour que l'utilisateur
    /// reconnaisse ses connexions dans `GET /api/v1/sessions/`.
    #[schema(max_length = 512, example = "Chrome 140 sur Windows 11")]
    device_info: String,
}

impl LoginView {
    #[must_use]
    pub fn email(&self) -> String {
        self.email.clone()
    }

    #[must_use]
    pub fn password(&self) -> String {
        self.password.clone()
    }

    #[must_use]
    pub fn device_info(&self) -> String {
        self.device_info.clone()
    }
}

impl Display for LoginView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginView {{ email: {}, password: {}, device_info: {} }}",
            self.email, self.password, self.device_info
        )
    }
}

/// Connexion réussie : jeton de rafraîchissement de la session ouverte.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginResponseView {
    /// Jeton opaque permettant d'obtenir un nouveau JWT via `POST /api/v1/sessions/refresh`,
    /// sans redemander le mot de passe. À conserver côté client, jamais dans une URL.
    #[schema(example = "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE")]
    refresh_token: String,
}

impl LoginResponseView {
    #[must_use]
    pub const fn new(refresh_token: String) -> Self {
        Self { refresh_token }
    }

    #[must_use]
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}

impl Display for LoginResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoginResponseView {{ refresh_token: {} }}",
            self.refresh_token
        )
    }
}

impl From<String> for LoginResponseView {
    fn from(token: String) -> Self {
        Self {
            refresh_token: token,
        }
    }
}

/// Première connexion : jeton à présenter pour choisir un nouveau mot de passe avant d'ouvrir une session.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginFirstConnectionResponseView {
    /// Jeton de première connexion, à usage unique, à présenter à
    /// `POST /api/v1/auth/force_change_password` pour choisir un mot de passe.
    #[schema(
        pattern = r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
        example = "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35"
    )]
    token: String,
}

impl LoginFirstConnectionResponseView {
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self { token }
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl Display for LoginFirstConnectionResponseView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ \"token\": {} }}", self.token)
    }
}

impl From<String> for LoginFirstConnectionResponseView {
    fn from(token: String) -> Self {
        Self { token }
    }
}

impl Validate for LoginView {
    fn validate(&self) -> Result<(), ValidationError> {
        // Only length and control characters: a malformed address simply matches no account (401).
        check_opaque("email", &self.email, MAX_EMAIL_LENGTH)?;
        check_opaque("password", &self.password, MAX_PASSWORD_LENGTH)?;
        check_opaque("device_info", &self.device_info, MAX_TOKEN_LENGTH)
    }
}
