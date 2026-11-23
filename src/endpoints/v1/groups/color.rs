//! Colour of a group (MAIR-923, Database `mair-923-02`): a name of the charter palette, shown on the
//! group cards of the administration. `NULL`: the front picks its default.
use crate::endpoints::validation::ValidationError;

/// Colours a group may take, as the Database `chk_groups_color` lists them.
pub const GROUP_COLORS: [&str; 7] = [
    "blue", "teal", "green", "orange", "purple", "pink", "neutral",
];

/// Checks a colour of the palette.
///
/// # Errors
///
/// Returns the `400` naming `color` when the value is not one of [`GROUP_COLORS`].
pub fn check_color(value: Option<&str>) -> Result<(), ValidationError> {
    match value {
        Some(color) if !GROUP_COLORS.contains(&color) => Err(ValidationError::new(
            "color",
            "must be one of blue, teal, green, orange, purple, pink, neutral",
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_palette_and_nothing_else() {
        for color in GROUP_COLORS {
            assert!(check_color(Some(color)).is_ok());
        }
        assert!(check_color(None).is_ok());
        assert!(check_color(Some("red")).is_err());
        assert!(check_color(Some("Blue")).is_err());
        assert!(check_color(Some("")).is_err());
    }
}
