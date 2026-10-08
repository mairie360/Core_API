//! Input helpers shared by the preference endpoints of `/api/v1/user/me`.

use serde::{Deserialize, Deserializer};

/// Deserializes a field where "absent" and `null` mean different things: combined with
/// `#[serde(default)]`, an absent field stays `None`, `null` becomes `Some(None)` and a value
/// `Some(Some(v))`.
///
/// # Errors
///
/// Returns the deserializer's error when the value is present but not a valid `T`.
#[allow(clippy::option_option)]
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Checks a setting restricted to a list of values.
///
/// # Errors
///
/// Returns the message of the `400` answer when the value is not in `allowed`.
pub fn check_one_of(
    field: &str,
    value: Option<Option<&str>>,
    allowed: &[&str],
) -> Result<(), String> {
    match value {
        Some(Some(value)) if !allowed.contains(&value) => {
            let allowed = allowed
                .iter()
                .map(|value| format!("`{value}`"))
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!("Invalid `{field}`: must be one of {allowed}"))
        }
        _ => Ok(()),
    }
}

/// Checks a setting with a format rule.
///
/// # Errors
///
/// Returns `Invalid `<field>`: <expected>` when the value breaks the rule.
pub fn check_rule(
    field: &str,
    value: Option<Option<&str>>,
    rule: fn(&str) -> bool,
    expected: &str,
) -> Result<(), String> {
    match value {
        Some(Some(value)) if !rule(value) => Err(format!("Invalid `{field}`: {expected}")),
        _ => Ok(()),
    }
}
