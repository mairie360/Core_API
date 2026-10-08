use crate::database::users::patch_preferences::PreferencesPatch;
use crate::endpoints::v1::user::me::nullable::{check_one_of, check_rule, double_option};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const THEMES: [&str; 3] = ["light", "dark", "system"];
/// Values of `density`, the ones the Settings front offers (MAIR-479).
pub const DENSITIES: [&str; 3] = ["compact", "normal", "comfortable"];
/// Values of `date_format`, the ones the Settings front offers (MAIR-479).
pub const DATE_FORMATS: [&str; 3] = ["DD/MM/YYYY", "MM/DD/YYYY", "YYYY-MM-DD"];
/// Values of `font_family` (MAIR-479): the DSFR font, two common sans-serif fonts, a font for
/// dyslexic readers and the system font.
pub const FONT_FAMILIES: [&str; 5] = ["Marianne", "Arial", "Verdana", "OpenDyslexic", "system-ui"];

/// Partial update of the display preferences. For every field: absent keeps the stored value,
/// `null` resets it to the application default, a value stores it.
#[allow(clippy::option_option)]
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct PatchPreferencesView {
    /// `light`, `dark` or `system`.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "dark")]
    theme: Option<Option<String>>,
    /// `Marianne`, `Arial`, `Verdana`, `OpenDyslexic` or `system-ui`.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Marianne")]
    font_family: Option<Option<String>>,
    /// Font size in pixels, 1 to 32767.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i32>, example = 16)]
    font_size: Option<Option<i32>>,
    /// `compact`, `normal` or `comfortable`.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "compact")]
    density: Option<Option<String>>,
    /// ISO 639 language code, optionally followed by an ISO 3166 region: `fr`, `en`, `fr-FR`
    /// (two or three lowercase letters, then `-` and two uppercase letters).
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, pattern = "^[a-z]{2,3}(-[A-Z]{2})?$", example = "fr")]
    language: Option<Option<String>>,
    /// IANA time zone name, case-sensitive: `Europe/Paris`, `America/Cayenne`, `UTC`.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 64, example = "Europe/Paris")]
    timezone: Option<Option<String>>,
    /// `DD/MM/YYYY`, `MM/DD/YYYY` or `YYYY-MM-DD`.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "DD/MM/YYYY")]
    date_format: Option<Option<String>>,
    /// Path of a page of the application: starts with `/`, then lowercase letters, digits, `-`,
    /// `_` and `/`, no `//`, 128 characters at most.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, max_length = 128, pattern = "^/[a-z0-9_/-]*$", example = "/dashboard")]
    home_page: Option<Option<String>>,
    /// Whether the notification panel opens by itself on new notifications.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = false)]
    auto_open_notifications: Option<Option<bool>>,
}

impl PatchPreferencesView {
    #[allow(clippy::option_option, clippy::ref_option)]
    fn text(field: &Option<Option<String>>) -> Option<Option<&str>> {
        field.as_ref().map(Option::as_deref)
    }

    /// Builds the database patch, borrowing the strings of the view.
    #[must_use]
    pub fn as_patch(&self) -> PreferencesPatch<'_> {
        PreferencesPatch {
            theme: Self::text(&self.theme),
            font_family: Self::text(&self.font_family),
            font_size: self.font_size,
            density: Self::text(&self.density),
            language: Self::text(&self.language),
            timezone: Self::text(&self.timezone),
            date_format: Self::text(&self.date_format),
            home_page: Self::text(&self.home_page),
            auto_open_notifications: self.auto_open_notifications,
        }
    }

    /// Checks the provided values against the values the application understands (MAIR-479):
    /// free text used to be stored as is, which let any client save values no front can read.
    ///
    /// # Errors
    ///
    /// Returns the message of the `400` answer for the first field breaking a rule.
    pub fn validate(&self) -> Result<(), String> {
        let patch = self.as_patch();
        if let Some(Some(theme)) = patch.theme {
            if !THEMES.contains(&theme) {
                return Err("Invalid `theme`: must be `light`, `dark` or `system`".to_string());
            }
        }
        if let Some(Some(size)) = patch.font_size {
            if !(1..=i32::from(i16::MAX)).contains(&size) {
                return Err("Invalid `font_size`: must be between 1 and 32767".to_string());
            }
        }
        check_one_of("font_family", patch.font_family, &FONT_FAMILIES)?;
        check_one_of("density", patch.density, &DENSITIES)?;
        check_one_of("date_format", patch.date_format, &DATE_FORMATS)?;
        check_rule(
            "language",
            patch.language,
            is_language_code,
            "must be a language code such as `fr` or `fr-FR`",
        )?;
        check_rule(
            "timezone",
            patch.timezone,
            is_time_zone,
            "must be an IANA time zone such as `Europe/Paris`",
        )?;
        check_rule(
            "home_page",
            patch.home_page,
            is_home_page,
            "must be a path of the application such as `/dashboard`",
        )?;
        Ok(())
    }
}

/// `fr`, `fra` or `fr-FR`: an ISO 639 code, optionally followed by an ISO 3166 region.
fn is_language_code(value: &str) -> bool {
    let (language, region) = value
        .split_once('-')
        .map_or((value, None), |(language, region)| (language, Some(region)));
    (2..=3).contains(&language.len())
        && language.bytes().all(|b| b.is_ascii_lowercase())
        && region.is_none_or(|region| {
            region.len() == 2 && region.bytes().all(|b| b.is_ascii_uppercase())
        })
}

/// A name of the IANA time zone database, as `chrono-tz` knows it (`Europe/Paris`, `UTC`).
fn is_time_zone(value: &str) -> bool {
    value.len() <= 64 && value.parse::<chrono_tz::Tz>().is_ok()
}

/// A path of the application: `/`, then lowercase letters, digits, `-`, `_` and `/`, no `//`.
fn is_home_page(value: &str) -> bool {
    value.len() <= 128
        && value.starts_with('/')
        && !value.contains("//")
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'/')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes() {
        for valid in ["fr", "en", "fra", "fr-FR", "en-GB"] {
            assert!(is_language_code(valid), "{valid}");
        }
        for invalid in [
            "", "f", "FR", "fr-fr", "fr_FR", "fr-FRA", "general", "fr-", "../fr",
        ] {
            assert!(!is_language_code(invalid), "{invalid}");
        }
    }

    #[test]
    fn time_zones() {
        for valid in ["Europe/Paris", "America/Cayenne", "Indian/Reunion", "UTC"] {
            assert!(is_time_zone(valid), "{valid}");
        }
        for invalid in [
            "",
            "general",
            "europe/paris",
            "Europe/Nowhere",
            "../../etc/passwd",
            "/etc/localtime",
        ] {
            assert!(!is_time_zone(invalid), "{invalid}");
        }
    }

    #[test]
    fn home_pages() {
        for valid in ["/", "/dashboard", "/projects/42", "/e-learning/my_courses"] {
            assert!(is_home_page(valid), "{valid}");
        }
        for invalid in [
            "",
            "general",
            "dashboard",
            "//evil.example",
            "/../etc/passwd",
            "/Dashboard",
            "/a?b=c",
            "https://evil.example",
        ] {
            assert!(!is_home_page(invalid), "{invalid}");
        }
        assert!(!is_home_page(&format!("/{}", "a".repeat(128))));
    }
}
