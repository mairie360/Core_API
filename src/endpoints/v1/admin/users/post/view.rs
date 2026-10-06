use crate::endpoints::validation::{
    check_email, check_label, check_password, check_phone, Validate, ValidationError,
    MAX_NAME_LENGTH, MIN_PASSWORD_LENGTH,
};
use crate::phone::Phone;
use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Compte utilisateur créé par un administrateur.
#[derive(Deserialize, ToSchema)]
pub struct CreateUserView {
    /// Prénom de l'utilisateur.
    #[schema(min_length = 1, max_length = 64, example = "Jean")]
    first_name: String,
    /// Nom de famille de l'utilisateur.
    #[schema(min_length = 1, max_length = 64, example = "Dupont")]
    last_name: String,
    /// Adresse e-mail, unique sur la plateforme. Doit contenir un `@` et un domaine pointé.
    #[schema(max_length = 320, format = Email, example = "jean.dupont@mairie360.fr")]
    email: String,
    /// Temporary password, 8 to 255 characters, no control character. The user must choose
    /// another one at their first login.
    #[schema(min_length = 8, max_length = 255, format = Password, example = "MotDePasse!123")]
    password: String,
    /// Optional phone number, as typed: national format (`06 12 34 56 78`) with `phone_country`,
    /// or E.164 (`+33612345678`, `phone_country` optional). It must be a valid number of that
    /// country. Absent, `null` or `""`: no phone.
    #[schema(max_length = 32, example = "06 12 34 56 78")]
    phone_number: Option<String>,
    /// ISO 3166-1 alpha-2 code of the country the number is typed for (`FR`, `BE`...). Required
    /// with a national number, refused without a number. The stored country is the number's own:
    /// a `+262` number sent with `FR` is stored `RE`.
    #[schema(min_length = 2, max_length = 2, pattern = "^[A-Z]{2}$", example = "FR")]
    #[serde(default)]
    phone_country: Option<String>,
}

impl CreateUserView {
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    /// The phone of the account, `None` when none was sent.
    ///
    /// # Errors
    ///
    /// Returns the `400` of an invalid phone or country (already refused by [`Validate`]).
    pub fn phone(&self) -> Result<Option<Phone>, ValidationError> {
        match self.phone_number.as_deref() {
            None | Some("") if self.phone_country.is_some() => Err(ValidationError::new(
                "phone_country",
                "must come with `phone_number`",
            )),
            None | Some("") => Ok(None),
            Some(number) => {
                check_phone("phone_number", number, self.phone_country.as_deref()).map(Some)
            }
        }
    }
}

impl Display for CreateUserView {
    // The e-mail, password and phone number never reach the logs (MAIR-225).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CreateUserView {{ first_name: {}, last_name: {}, email: [PROTECTED], password: [PROTECTED], phone_number: {} }}",
            self.first_name,
            self.last_name,
            if self.phone_number.is_some() { "[PROTECTED]" } else { "none" }
        )
    }
}

impl Validate for CreateUserView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("first_name", &self.first_name, MAX_NAME_LENGTH)?;
        check_label("last_name", &self.last_name, MAX_NAME_LENGTH)?;
        check_email("email", &self.email)?;
        check_password("password", &self.password, MIN_PASSWORD_LENGTH)?;
        self.phone().map(|_| ())
    }
}
