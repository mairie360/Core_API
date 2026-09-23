use crate::endpoints::validation::{
    check_email, check_label, check_optional, check_password, check_phone, Validate,
    ValidationError, MAX_NAME_LENGTH, MIN_PASSWORD_LENGTH,
};
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
    /// Optional phone number. When present: 10 to 15 digits only.
    #[schema(pattern = "^[0-9]{10,15}$", example = "0612345678")]
    phone_number: Option<String>,
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

    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }
}

impl Display for CreateUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CreateUserView {{ first_name: {}, last_name: {}, email: {}, password: {}, phone_number: {:?} }}",
            self.first_name, self.last_name, self.email, self.password, self.phone_number
        )
    }
}

impl Validate for CreateUserView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("first_name", &self.first_name, MAX_NAME_LENGTH)?;
        check_label("last_name", &self.last_name, MAX_NAME_LENGTH)?;
        check_email("email", &self.email)?;
        check_password("password", &self.password, MIN_PASSWORD_LENGTH)?;
        check_optional(self.phone_number.as_deref(), |phone| {
            check_phone("phone_number", phone)
        })
    }
}
