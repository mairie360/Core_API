//! Input helpers shared by the preference endpoints of `/api/v1/user/me`.

use serde::{Deserialize, Deserializer};

/// Deserializes a field where "absent" and `null` mean different things: combined with
/// `#[serde(default)]`, an absent field stays `None`, `null` becomes `Some(None)` and a value
/// `Some(Some(v))`.
#[allow(clippy::option_option)]
pub fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Checks a free text setting: 1 to `max` characters, no control character, no `<` or `>`.
///
/// # Errors
///
/// Returns the message of the `400` answer when the value breaks a rule.
pub fn check_text(field: &str, value: Option<Option<&str>>, max: usize) -> Result<(), String> {
    let Some(Some(value)) = value else {
        return Ok(());
    };
    let length = value.chars().count();
    if value.trim().is_empty() || length > max {
        return Err(format!(
            "Invalid `{field}`: must be 1 to {max} characters, not blank"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "Invalid `{field}`: must not contain control characters"
        ));
    }
    if value.contains(['<', '>']) {
        return Err(format!("Invalid `{field}`: must not contain `<` or `>`"));
    }
    Ok(())
}
