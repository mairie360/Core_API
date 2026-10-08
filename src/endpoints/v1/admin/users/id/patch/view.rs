use crate::endpoints::v1::user::me::nullable::double_option;
use crate::endpoints::validation::{
    check_email, check_label, check_optional, check_password, check_phone_change, Validate,
    ValidationError, MAX_NAME_LENGTH, MIN_PASSWORD_LENGTH,
};
use crate::phone::PhoneChange;
use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Modification partielle d'un utilisateur par un administrateur : seuls les champs fournis sont mis à jour.
#[allow(clippy::option_option)]
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
    /// New phone number, as typed: national format (`07 98 76 54 32`) with `phone_country`, or
    /// E.164 (`+33798765432`, `phone_country` optional). It must be a valid number of that
    /// country. Absent keeps the stored phone; `null` or `""` removes it.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 32, example = "07 98 76 54 32")]
    phone_number: Option<Option<String>>,
    /// ISO 3166-1 alpha-2 code of the country the number is typed for (`FR`, `BE`...). Required
    /// with a national number, refused without a number. The stored country is the number's own:
    /// a `+262` number sent with `FR` is stored `RE`.
    #[schema(min_length = 2, max_length = 2, pattern = "^[A-Z]{2}$", example = "FR")]
    #[serde(default)]
    phone_country: Option<String>,
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

    /// What the request does to the phone.
    ///
    /// # Errors
    ///
    /// Returns the `400` of an invalid phone or country (already refused by [`Validate`]).
    pub fn phone_change(&self) -> Result<PhoneChange, ValidationError> {
        check_phone_change(
            "phone_number",
            self.phone_number.as_ref().map(Option::as_deref),
            self.phone_country.as_deref(),
        )
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
}

impl Display for PatchUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchUserView {{ first_name: {}, last_name: {}, email: {}, phone_number: {}, password: {} }}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or(""),
            if self.email.is_some() { "[PROTECTED]" } else { "unchanged" },
            if self.phone_number.is_some() { "[PROTECTED]" } else { "unchanged" },
            if self.password.is_some() { "[PROTECTED]" } else { "unchanged" }
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
        self.phone_change()?;
        check_optional(self.password.as_deref(), |password| {
            check_password("password", password, MIN_PASSWORD_LENGTH)
        })
    }
}
