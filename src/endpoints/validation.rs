//! Input validation shared by every request body and query string of the API.
//!
//! A request view implements [`Validate`] and the handler extracts it with [`ValidatedJson`] or
//! [`ValidatedQuery`] instead of `web::Json` / `web::Query`: an invalid value is rejected with a
//! `400 Bad Request` (plain-text body naming the field) before the handler runs, so it never
//! reaches Postgres (where an over-long value or a NUL byte used to end in a `500`) nor comes back
//! unescaped in a JSON response.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

use actix_web::{dev::Payload, web, FromRequest, HttpRequest};

use crate::phone::{Phone, PhoneChange};
use serde::de::DeserializeOwned;

/// `users.first_name`, `users.last_name`, `roles.name` and `groups.name` are `VARCHAR(64)`.
pub const MAX_NAME_LENGTH: usize = 64;
/// `users.email` is `VARCHAR(320)` (RFC 5321 maximum).
pub const MAX_EMAIL_LENGTH: usize = 320;
/// Role and group descriptions (`TEXT` columns, capped to keep the payloads reasonable).
pub const MAX_DESCRIPTION_LENGTH: usize = 1000;
/// Passwords, as typed by the user, on the routes that create an account or set a password.
pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 255;
/// Opaque values the API issued itself (refresh tokens, one-time tokens) and client-provided
/// labels (`device_info`).
pub const MAX_TOKEN_LENGTH: usize = 512;
/// Free-text search filters of the listing routes.
pub const MAX_SEARCH_LENGTH: usize = 255;

/// Why a request value was rejected; its text is the body of the `400` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(String);

impl ValidationError {
    pub(crate) fn new(field: &str, reason: &str) -> Self {
        Self(format!("Invalid `{field}`: {reason}"))
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Implemented by every request view extracted with [`ValidatedJson`] or [`ValidatedQuery`].
pub trait Validate {
    /// # Errors
    ///
    /// Returns the first field that does not satisfy its constraints.
    fn validate(&self) -> Result<(), ValidationError>;
}

fn check_length(field: &str, value: &str, max: usize) -> Result<(), ValidationError> {
    if value.chars().count() > max {
        return Err(ValidationError::new(
            field,
            &format!("must be at most {max} characters"),
        ));
    }
    Ok(())
}

fn check_no_control(field: &str, value: &str) -> Result<(), ValidationError> {
    if value.chars().any(char::is_control) {
        return Err(ValidationError::new(
            field,
            "must not contain control characters",
        ));
    }
    Ok(())
}

/// A short label (person name, role or group name): not blank, at most `max` characters, no
/// control character.
///
/// `<` and `>` are accepted (MAIR-426): the API serves JSON with `nosniff`, and escaping text for
/// HTML is the fronts' job (React escapes it), so "budget > 10 000 €" or "<3" are valid names.
///
/// # Errors
///
/// Returns a [`ValidationError`] naming `field` when one of the rules is broken.
pub fn check_label(field: &str, value: &str, max: usize) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new(field, "must not be empty"));
    }
    check_length(field, value, max)?;
    check_no_control(field, value)
}

/// A free-text description: may be empty, at most `max` characters, line breaks and tabs
/// allowed, no other control character.
///
/// # Errors
///
/// Returns a [`ValidationError`] naming `field` when one of the rules is broken.
pub fn check_description(field: &str, value: &str, max: usize) -> Result<(), ValidationError> {
    check_length(field, value, max)?;
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(ValidationError::new(
            field,
            "must not contain control characters other than line breaks and tabs",
        ));
    }
    Ok(())
}

/// An e-mail address that can be written to `users.email` and used as a mail recipient.
///
/// # Errors
///
/// Returns a [`ValidationError`] naming `field` when the address is too long or malformed.
pub fn check_email(field: &str, value: &str) -> Result<(), ValidationError> {
    check_length(field, value, MAX_EMAIL_LENGTH)?;
    let domain_has_dot = value
        .rsplit_once('@')
        .is_some_and(|(_, domain)| domain.contains('.'));
    if !domain_has_dot || lettre::Address::from_str(value).is_err() {
        return Err(ValidationError::new(
            field,
            "must be a valid e-mail address",
        ));
    }
    Ok(())
}

/// A phone number as typed, with the country picked for it (MAIR-480), see [`Phone::parse`].
///
/// # Errors
///
/// Returns a [`ValidationError`] naming `phone_country` when the country is not an ISO 3166-1
/// alpha-2 code, `field` when the number is not valid for it.
pub fn check_phone(
    field: &str,
    number: &str,
    country: Option<&str>,
) -> Result<Phone, ValidationError> {
    if let Some(country) = country {
        crate::phone::check_country(country)
            .map_err(|reason| ValidationError::new("phone_country", reason))?;
    }
    Phone::parse(country, number).map_err(|reason| ValidationError::new(field, reason))
}

/// What a partial update does to the phone (MAIR-480): `number` absent keeps it, `null` or `""`
/// clears it, a value replaces it. `phone_country` alone is refused: it qualifies a number.
///
/// # Errors
///
/// Returns the [`ValidationError`] of [`check_phone`], or one naming `phone_country` when it is
/// sent without a number.
#[allow(clippy::option_option)]
pub fn check_phone_change(
    field: &str,
    number: Option<Option<&str>>,
    country: Option<&str>,
) -> Result<PhoneChange, ValidationError> {
    match number {
        None if country.is_some() => Err(ValidationError::new(
            "phone_country",
            &format!("must come with `{field}`"),
        )),
        None => Ok(PhoneChange::Keep),
        Some(None | Some("")) => Ok(PhoneChange::Clear),
        Some(Some(number)) => check_phone(field, number, country).map(PhoneChange::Set),
    }
}

/// A password: between `min` and [`MAX_PASSWORD_LENGTH`] characters, no control character.
///
/// # Errors
///
/// Returns a [`ValidationError`] naming `field` when one of the rules is broken.
pub fn check_password(field: &str, value: &str, min: usize) -> Result<(), ValidationError> {
    if value.chars().count() < min {
        return Err(ValidationError::new(
            field,
            &format!("must be at least {min} characters"),
        ));
    }
    check_length(field, value, MAX_PASSWORD_LENGTH)?;
    check_no_control(field, value)
}

/// An opaque value only compared or stored as text (token, credential, `device_info`, search
/// filter): at most `max` characters and no control character (Postgres rejects NUL bytes).
///
/// # Errors
///
/// Returns a [`ValidationError`] naming `field` when one of the rules is broken.
pub fn check_opaque(field: &str, value: &str, max: usize) -> Result<(), ValidationError> {
    check_length(field, value, max)?;
    check_no_control(field, value)
}

/// Runs `check` on `value` when it is present.
///
/// # Errors
///
/// Returns the error of `check`.
pub fn check_optional<F>(value: Option<&str>, check: F) -> Result<(), ValidationError>
where
    F: FnOnce(&str) -> Result<(), ValidationError>,
{
    value.map_or(Ok(()), check)
}

fn bad_request(error: &ValidationError) -> actix_web::Error {
    actix_web::error::ErrorBadRequest(error.to_string())
}

/// `web::Json<T>` followed by [`Validate::validate`]: answers `400` when either fails.
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> FromRequest for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + 'static,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let json = web::Json::<T>::from_request(req, payload);
        Box::pin(async move {
            let value = json.await?.into_inner();
            value.validate().map_err(|e| bad_request(&e))?;
            Ok(Self(value))
        })
    }
}

/// `web::Query<T>` followed by [`Validate::validate`]: answers `400` when either fails.
pub struct ValidatedQuery<T>(pub T);

impl<T> ValidatedQuery<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> FromRequest for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate + 'static,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let query = web::Query::<T>::from_query(req.query_string());
        Box::pin(async move {
            let value = query?.into_inner();
            value.validate().map_err(|e| bad_request(&e))?;
            Ok(Self(value))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_rejects_blank_long_and_control() {
        assert!(check_label("name", "Jean", MAX_NAME_LENGTH).is_ok());
        assert!(check_label("name", "  ", MAX_NAME_LENGTH).is_err());
        assert!(check_label("name", &"a".repeat(65), MAX_NAME_LENGTH).is_err());
        assert!(check_label("name", "Je\0an", MAX_NAME_LENGTH).is_err());
        assert!(check_label("name", "Budget > 10 000 € <3", MAX_NAME_LENGTH).is_ok());
    }

    #[test]
    fn label_counts_characters_not_bytes() {
        assert!(check_label("name", &"é".repeat(64), MAX_NAME_LENGTH).is_ok());
    }

    #[test]
    fn description_allows_line_breaks_only() {
        assert!(check_description("description", "", MAX_DESCRIPTION_LENGTH).is_ok());
        assert!(check_description("description", "a\nb\tc", MAX_DESCRIPTION_LENGTH).is_ok());
        assert!(check_description("description", "a\0b", MAX_DESCRIPTION_LENGTH).is_err());
        assert!(check_description(
            "description",
            "Budget > 10 000 € -> <b>",
            MAX_DESCRIPTION_LENGTH
        )
        .is_ok());
    }

    #[test]
    fn email_rules() {
        assert!(check_email("email", "jean.dupont@mairie360.fr").is_ok());
        assert!(check_email("email", "jean.dupont@mairie360").is_err());
        assert!(check_email("email", "http://\\1.owasp.org").is_err());
        assert!(check_email("email", "jean.dupont@mairie360.fr AND 1=1 -- ").is_err());
        let long = format!("{}@mairie360.fr", "a".repeat(320));
        assert!(check_email("email", &long).is_err());
    }

    #[test]
    fn phone_rules() {
        assert!(check_phone("phone", "0798765432", Some("FR")).is_ok());
        assert!(check_phone("phone", "079876543", Some("FR")).is_err());
        assert!(check_phone("phone", "0798765432 AND 1=1 -- ", Some("FR")).is_err());
        assert!(check_phone("phone", "<script>alert(1);</script>", Some("FR")).is_err());
        let wrong_country = check_phone("phone", "0798765432", Some("France")).unwrap_err();
        assert!(wrong_country
            .to_string()
            .starts_with("Invalid `phone_country`"));
    }

    #[test]
    fn phone_changes() {
        assert_eq!(
            check_phone_change("phone", None, None),
            Ok(PhoneChange::Keep)
        );
        assert_eq!(
            check_phone_change("phone", Some(None), None),
            Ok(PhoneChange::Clear)
        );
        assert_eq!(
            check_phone_change("phone", Some(Some("")), Some("FR")),
            Ok(PhoneChange::Clear)
        );
        assert!(matches!(
            check_phone_change("phone", Some(Some("0612345678")), Some("FR")),
            Ok(PhoneChange::Set(_))
        ));
        assert!(check_phone_change("phone", None, Some("FR")).is_err());
        assert!(check_phone_change("phone", Some(Some("0612345678")), None).is_err());
    }

    #[test]
    fn password_and_opaque_reject_nul() {
        assert!(check_password("password", "MotDePasse!123", 8).is_ok());
        assert!(check_password("password", "court", 8).is_err());
        assert!(check_password("password", "MotDe\0Passe!123", 8).is_err());
        assert!(check_opaque("refresh_token", "abc\0", MAX_TOKEN_LENGTH).is_err());
        assert!(check_opaque("refresh_token", "Zm9vYmFy", MAX_TOKEN_LENGTH).is_ok());
    }

    #[test]
    fn optional_skips_absent_values() {
        assert!(check_optional(None, |v| check_label("name", v, MAX_NAME_LENGTH)).is_ok());
        assert!(check_optional(Some(""), |v| check_label("name", v, MAX_NAME_LENGTH)).is_err());
    }
}
