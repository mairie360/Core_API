use crate::endpoints::validation::{
    check_email, check_label, check_optional, check_password, check_phone, Validate,
    ValidationError, MAX_NAME_LENGTH, MIN_PASSWORD_LENGTH,
};
use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Modification partielle d'un utilisateur par un administrateur : seuls les champs fournis sont mis à jour.
#[derive(Deserialize, ToSchema)]
pub struct PatchUserView {
    /// Nouveau prénom. Absent ou `null` pour ne pas y toucher.
    #[schema(min_length = 1, max_length = 64, example = "Jean")]
    first_name: Option<String>,
    /// Nouveau nom de famille. Absent ou `null` pour ne pas y toucher.
    #[schema(min_length = 1, max_length = 64, example = "Dupont")]
    last_name: Option<String>,
    /// Nouvelle adresse e-mail. Elle doit rester unique, sinon l'appel échoue en `404`.
    #[schema(max_length = 320, format = Email, example = "j.dupont@mairie360.fr")]
    email: Option<String>,
    /// Nouveau numéro de téléphone. Absent ou `null` pour ne pas y toucher.
    #[schema(pattern = "^[0-9]{10,15}$", example = "0798765432")]
    phone_number: Option<String>,
    /// New password, 8 to 255 characters, no control character. Absent or `null` to leave it
    /// unchanged. `PATCH /api/v1/admin/users/{userId}/password` is the dedicated route.
    #[schema(min_length = 8, max_length = 255, format = Password, example = "NouveauMotDePasse!123")]
    password: Option<String>,
}

impl PatchUserView {
    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }

    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl Display for PatchUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchUserView {{ first_name: {}, last_name: {}, email: {}, phone_number: {:?}, password: {:?} }}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or(""),
            self.email.as_deref().unwrap_or(""),
            self.phone_number.as_deref().unwrap_or(""),
            self.password.as_deref().unwrap_or("")
        )
    }
}

impl Validate for PatchUserView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.first_name.as_deref(), |name| {
            check_label("first_name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.last_name.as_deref(), |name| {
            check_label("last_name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.email.as_deref(), |email| check_email("email", email))?;
        check_optional(self.phone_number.as_deref(), |phone| {
            check_phone("phone_number", phone)
        })?;
        check_optional(self.password.as_deref(), |password| {
            check_password("password", password, MIN_PASSWORD_LENGTH)
        })
    }
}
