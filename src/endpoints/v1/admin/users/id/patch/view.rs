use crate::endpoints::v1::user::me::nullable::double_option;
use crate::endpoints::validation::{
    check_email, check_label, check_optional, check_phone_change, Validate, ValidationError,
    MAX_NAME_LENGTH,
};
use crate::phone::PhoneChange;
use serde::Deserialize;
use std::fmt::Display;
use utoipa::ToSchema;

/// Partial update of a user by an administrator: only the given fields are updated. Unknown
/// fields are refused, `password` included: an administrator never sets the password of an
/// account.
#[allow(clippy::option_option)]
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PatchUserView {
    /// New first name. Absent or `null` to leave it unchanged.
    #[schema(min_length = 1, max_length = 64, example = "Jean")]
    first_name: Option<String>,
    /// New last name. Absent or `null` to leave it unchanged.
    #[schema(min_length = 1, max_length = 64, example = "Dupont")]
    last_name: Option<String>,
    /// New e-mail address. It must stay unique, otherwise the call fails with `404`.
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
}

impl Display for PatchUserView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchUserView {{ first_name: {}, last_name: {}, email: {}, phone_number: {} }}",
            self.first_name.as_deref().unwrap_or(""),
            self.last_name.as_deref().unwrap_or(""),
            if self.email.is_some() {
                "[PROTECTED]"
            } else {
                "unchanged"
            },
            if self.phone_number.is_some() {
                "[PROTECTED]"
            } else {
                "unchanged"
            }
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
        self.phone_change().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Result<PatchUserView, serde_json::Error> {
        serde_json::from_str(body)
    }

    #[test]
    fn an_administrator_never_sends_a_password() {
        let refused = parse(r#"{"password":"x"}"#)
            .err()
            .expect("password is not a field of the admin patch");
        assert!(refused.to_string().contains("unknown field `password`"));
    }

    #[test]
    fn display_hides_the_email_and_the_phone() {
        let view = parse(
            r#"{"first_name":"Jean","email":"jean@mairie360.fr","phone_number":"+33612345678"}"#,
        )
        .unwrap();
        let shown = view.to_string();
        assert_eq!(
            shown,
            "PatchUserView { first_name: Jean, last_name: , email: [PROTECTED], phone_number: [PROTECTED] }"
        );
        assert!(!shown.contains("jean@mairie360.fr") && !shown.contains("612345678"));

        let unchanged = parse(r#"{"last_name":"Dupont"}"#).unwrap().to_string();
        assert_eq!(
            unchanged,
            "PatchUserView { first_name: , last_name: Dupont, email: unchanged, phone_number: unchanged }"
        );
    }

    #[test]
    fn validate_checks_each_given_field() {
        assert!(
            parse(r#"{"first_name":"Jean","email":"jean@mairie360.fr"}"#)
                .unwrap()
                .validate()
                .is_ok()
        );
        assert!(parse(r#"{"first_name":""}"#).unwrap().validate().is_err());
        assert!(parse(r#"{"email":"not-an-address"}"#)
            .unwrap()
            .validate()
            .is_err());
        assert!(parse(r#"{"phone_country":"FR"}"#)
            .unwrap()
            .validate()
            .is_err());
    }
}
