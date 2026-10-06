//! Phone numbers (MAIR-480).
//!
//! A phone is stored as two `users` columns (Database v1.10.0): `phone_country`, the ISO 3166-1
//! alpha-2 code of the number's country, and `phone_number`, its national significant number
//! (digits only, no trunk prefix: the French `06 12 34 56 78` is stored `FR` / `612345678`).
//!
//! Requests send the number as typed, in national format (`06 12 34 56 78`) or in E.164
//! (`+33612345678`), with its country; it is validated against the numbering plan of that
//! country. Responses give the E.164 form, ready to display or dial, next to the country.

use phonenumber::country::Id;
use phonenumber::{Mode, PhoneNumber};

/// Longest number accepted as typed (spaces, dots, dashes and parentheses included).
pub const MAX_PHONE_INPUT_LENGTH: usize = 32;

/// What a partial update does to the phone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhoneChange {
    /// The phone was not in the request: left as stored.
    Keep,
    /// `null` or `""`: the phone is removed.
    Clear,
    /// A new phone.
    Set(Phone),
}

/// A valid phone number, split the way `users` stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phone {
    country: String,
    national: String,
}

impl Phone {
    /// Parses `number` as typed by the user, `country` being the country they picked.
    ///
    /// The country of the number wins over the one picked when they differ but share the same
    /// calling code (`+262 692…` picked as `FR` is stored `RE`). A number in E.164 needs no
    /// country.
    ///
    /// # Errors
    ///
    /// Returns why the number is refused, worded for the `400` answer.
    pub fn parse(country: Option<&str>, number: &str) -> Result<Self, &'static str> {
        if number.chars().count() > MAX_PHONE_INPUT_LENGTH
            || !number
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '.' | '-' | '(' | ')'))
        {
            return Err("must be a phone number: digits, spaces, `.`, `-`, `(`, `)` and a leading `+`, 32 characters at most");
        }
        let country = match country {
            Some(country) => Some(parse_country(country)?),
            None if number.starts_with('+') => None,
            None => {
                return Err("needs `phone_country` unless it is written in E.164 (`+33612345678`)")
            }
        };
        let parsed = phonenumber::parse(country, number)
            .map_err(|_| "is not a phone number of the selected country")?;
        if !parsed.is_valid() {
            return Err("is not a phone number of the selected country");
        }
        let country = number_country(&parsed, country)
            .ok_or("is not a phone number of the selected country")?;
        Ok(Self {
            country: country.as_ref().to_string(),
            national: parsed.national().to_string(),
        })
    }

    /// ISO 3166-1 alpha-2 code of the number's country (`users.phone_country`).
    #[must_use]
    pub fn country(&self) -> &str {
        &self.country
    }

    /// National significant number, digits only (`users.phone_number`).
    #[must_use]
    pub fn national(&self) -> &str {
        &self.national
    }
}

/// The country of a valid number.
///
/// `PhoneNumber::country().id()` looks the region up without the leading zeros of the national
/// number, so it finds none for Italian landlines (`+39 06…`, a calling code shared with the
/// Vatican). The regions of the calling code are the fallback: the picked one when it is among
/// them, the main one otherwise.
fn number_country(number: &PhoneNumber, picked: Option<Id>) -> Option<Id> {
    number.country().id().or_else(|| {
        let regions = phonenumber::metadata::DATABASE.region(&number.country().code())?;
        picked
            .filter(|picked| regions.contains(&picked.as_ref()))
            .or_else(|| regions.first().and_then(|region| region.parse().ok()))
    })
}

/// Checks a country picked by the user.
///
/// # Errors
///
/// Returns why the country is refused, worded for the `400` answer.
pub fn check_country(country: &str) -> Result<(), &'static str> {
    parse_country(country).map(|_| ())
}

/// A country picked by the user: an ISO 3166-1 alpha-2 code known to the numbering plans.
fn parse_country(country: &str) -> Result<Id, &'static str> {
    if country.len() != 2 || !country.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err("must be an ISO 3166-1 alpha-2 country code such as `FR`");
    }
    country
        .parse::<Id>()
        .map_err(|_| "must be an ISO 3166-1 alpha-2 country code such as `FR`")
}

/// The phone of a `users` row as the API returns it.
///
/// The E.164 form when the country is known (`+33612345678`), the stored digits as they are
/// otherwise (a legacy number the Database v1.10.0 backfill could not attribute to a country).
#[must_use]
pub fn display(country: Option<&str>, national: Option<&str>) -> Option<String> {
    let national = national?;
    let e164 = country
        .and_then(|country| country.parse::<Id>().ok())
        .and_then(|country| phonenumber::parse(Some(country), national).ok())
        .map(|number: PhoneNumber| number.format().mode(Mode::E164).to_string());
    Some(e164.unwrap_or_else(|| national.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(country: Option<&str>, number: &str) -> Result<(String, String), &'static str> {
        Phone::parse(country, number).map(|phone| (phone.country, phone.national))
    }

    #[test]
    fn national_and_international_numbers() {
        let french = Ok(("FR".to_string(), "612345678".to_string()));
        assert_eq!(parse(Some("FR"), "0612345678"), french);
        assert_eq!(parse(Some("FR"), "06 12 34 56 78"), french);
        assert_eq!(parse(Some("FR"), "06.12.34.56.78"), french);
        assert_eq!(parse(Some("FR"), "+33 6 12 34 56 78"), french);
        assert_eq!(parse(None, "+33612345678"), french);
        assert_eq!(
            parse(Some("BE"), "0470 12 34 56"),
            Ok(("BE".to_string(), "470123456".to_string()))
        );
        // The country of the number wins: La Réunion shares nothing with `FR` but the plan.
        assert_eq!(
            parse(Some("FR"), "+262 692 12 34 56"),
            Ok(("RE".to_string(), "692123456".to_string()))
        );
        // Italian landlines keep their leading zero in the national number.
        assert_eq!(
            parse(Some("IT"), "06 6982 1234"),
            Ok(("IT".to_string(), "0669821234".to_string()))
        );
        assert_eq!(
            parse(None, "+39 06 6982 1234"),
            Ok(("IT".to_string(), "0669821234".to_string()))
        );
        // A calling code shared by several countries keeps the number's own country.
        assert_eq!(
            parse(Some("US"), "+1 604 555 0123"),
            Ok(("CA".to_string(), "6045550123".to_string()))
        );
    }

    #[test]
    fn refused_numbers() {
        for (country, number) in [
            (Some("FR"), "0612"),
            (Some("FR"), "06123456789012"),
            (Some("FR"), "0012345678"),
            (None, "0612345678"),
            (Some("fr"), "0612345678"),
            (Some("ZZ"), "0612345678"),
            (Some("FRA"), "0612345678"),
            (Some("FR"), "06 12 34 56 7a"),
            (Some("FR"), "0612345678 AND 1=1"),
            (Some("FR"), "<script>alert(1)</script>"),
            (Some("FR"), "../../etc/passwd"),
            (Some("FR"), &"0".repeat(33)),
        ] {
            assert!(parse(country, number).is_err(), "{country:?} {number}");
        }
    }

    #[test]
    fn display_in_e164() {
        assert_eq!(
            display(Some("FR"), Some("612345678")).as_deref(),
            Some("+33612345678")
        );
        assert_eq!(
            display(Some("IT"), Some("0669821234")).as_deref(),
            Some("+390669821234")
        );
        assert_eq!(display(Some("FR"), None), None);
        assert_eq!(display(None, None), None);
        // A legacy number without a country is returned as stored.
        assert_eq!(
            display(None, Some("4412345678")).as_deref(),
            Some("4412345678")
        );
    }
}
