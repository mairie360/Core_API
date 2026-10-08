use crate::endpoints::v1::user::me::nullable::double_option;
use crate::endpoints::validation::{
    check_email, check_label, check_opaque, check_optional, check_phone_change, Validate,
    ValidationError, MAX_NAME_LENGTH, MAX_PASSWORD_LENGTH,
};
use crate::phone::PhoneChange;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Partial update of the connected user's profile: only the fields sent are written.
#[allow(clippy::option_option)]
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
    /// New phone number, as typed: national format (`06 12 34 56 78`, digits, spaces, `.`, `-`,
    /// `(`, `)`) with `phone_country`, or E.164 (`+33612345678`, `phone_country` optional). It
    /// must be a valid number of that country. Absent keeps the stored phone; `null` or `""`
    /// removes it.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 32, example = "06 12 34 56 78")]
    phone: Option<Option<String>>,
    /// ISO 3166-1 alpha-2 code of the country `phone` is typed for (`FR`, `BE`...). Required with
    /// a national number, refused without `phone`. The stored country is the number's own: a
    /// `+262` number sent with `FR` is stored `RE`.
    #[schema(min_length = 2, max_length = 2, pattern = "^[A-Z]{2}$", example = "FR")]
    #[serde(default)]
    phone_country: Option<String>,
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
        phone: Option<Option<String>>,
        phone_country: Option<String>,
    ) -> Self {
        Self {
            first_name,
            last_name,
            email,
            phone,
            phone_country,
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

    /// What the request does to the phone.
    ///
    /// # Errors
    ///
    /// Returns the `400` of an invalid phone or country (already refused by [`Validate`]).
    pub fn phone_change(&self) -> Result<PhoneChange, ValidationError> {
        check_phone_change(
            "phone",
            self.phone.as_ref().map(Option::as_deref),
            self.phone_country.as_deref(),
        )
    }
}

impl Display for PatchMeView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchMeView {{ first_name: {:?}, last_name: {:?}, email: [PROTECTED], phone: {} }}",
            self.first_name,
            self.last_name,
            if self.phone.is_some() {
                "[PROTECTED]"
            } else {
                "unchanged"
            }
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
        self.phone_change()?;
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
