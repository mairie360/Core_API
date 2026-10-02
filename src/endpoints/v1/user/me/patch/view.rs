use crate::endpoints::validation::{
    check_email, check_label, check_opaque, check_optional, check_phone, Validate, ValidationError,
    MAX_NAME_LENGTH, MAX_PASSWORD_LENGTH,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Modification partielle du profil de l'utilisateur connecté : seuls les champs fournis sont mis à jour.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PatchMeView {
    /// Nouveau prénom. Absent ou `null` pour ne pas y toucher.
    #[schema(min_length = 1, max_length = 64, example = "Jean")]
    first_name: Option<String>,
    /// Nouveau nom de famille. Absent ou `null` pour ne pas y toucher.
    #[schema(min_length = 1, max_length = 64, example = "Dupont")]
    last_name: Option<String>,
    /// Nouvelle adresse e-mail. Absent ou `null` pour ne pas y toucher.
    #[schema(max_length = 320, format = Email, example = "jean.dupont@mairie360.fr")]
    email: Option<String>,
    /// Nouveau numéro de téléphone. Absent ou `null` pour ne pas y toucher.
    #[schema(pattern = "^[0-9]{10,15}$", example = "0798765432")]
    phone: Option<String>,
    /// Current password of the account. Required when `email` is sent (MAIR-390): a stolen
    /// session must not be enough to move the account to another mailbox. Ignored otherwise.
    #[schema(max_length = 255, format = Password, example = "MotDePasse!123")]
    #[serde(default)]
    current_password: Option<String>,
}

impl PatchMeView {
    #[must_use]
    pub const fn new(
        first_name: Option<String>,
        last_name: Option<String>,
        email: Option<String>,
        phone: Option<String>,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email,
            phone,
            current_password: None,
        }
    }

    #[must_use]
    pub fn current_password(&self) -> Option<&str> {
        self.current_password.as_deref()
    }

    #[must_use]
    pub fn first_name(&self) -> Option<&str> {
        self.first_name.as_deref()
    }

    #[must_use]
    pub fn last_name(&self) -> Option<&str> {
        self.last_name.as_deref()
    }

    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    #[must_use]
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }
}

impl Display for PatchMeView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchMeView {{ first_name: {:?}, last_name: {:?}, email: {:?}, phone: {:?} }}",
            self.first_name, self.last_name, self.email, self.phone
        )
    }
}

impl Validate for PatchMeView {
    fn validate(&self) -> Result<(), ValidationError> {
        check_optional(self.first_name.as_deref(), |name| {
            check_label("first_name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.last_name.as_deref(), |name| {
            check_label("last_name", name, MAX_NAME_LENGTH)
        })?;
        check_optional(self.email.as_deref(), |email| check_email("email", email))?;
        check_optional(self.phone.as_deref(), |phone| check_phone("phone", phone))?;
        if self.email.is_some() && self.current_password.is_none() {
            return Err(ValidationError::new(
                "current_password",
                "is required to change the e-mail address",
            ));
        }
        check_optional(self.current_password.as_deref(), |password| {
            check_opaque("current_password", password, MAX_PASSWORD_LENGTH)
        })
    }
}
