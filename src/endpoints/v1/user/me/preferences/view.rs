use crate::database::users::patch_preferences::PreferencesPatch;
use crate::endpoints::v1::user::me::nullable::{check_text, double_option};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const THEMES: [&str; 3] = ["light", "dark", "system"];

/// Partial update of the display preferences. For every field: absent keeps the stored value,
/// `null` resets it to the application default, a value stores it.
#[allow(clippy::option_option)]
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct PatchPreferencesView {
    /// `light`, `dark` or `system`.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "dark")]
    theme: Option<Option<String>>,
    /// 1 to 128 characters.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Marianne")]
    font_family: Option<Option<String>>,
    /// Font size in pixels, 1 to 32767.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<i32>, example = 16)]
    font_size: Option<Option<i32>>,
    /// 1 to 32 characters.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "compact")]
    density: Option<Option<String>>,
    /// 1 to 16 characters.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "fr")]
    language: Option<Option<String>>,
    /// 1 to 64 characters.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Europe/Paris")]
    timezone: Option<Option<String>>,
    /// 1 to 32 characters.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "DD/MM/YYYY")]
    date_format: Option<Option<String>>,
    /// 1 to 128 characters.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "/dashboard")]
    home_page: Option<Option<String>>,
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

    /// Checks the provided values against the constraints of the `user_preferences` columns.
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
        check_text("font_family", patch.font_family, 128)?;
        check_text("density", patch.density, 32)?;
        check_text("language", patch.language, 16)?;
        check_text("timezone", patch.timezone, 64)?;
        check_text("date_format", patch.date_format, 32)?;
        check_text("home_page", patch.home_page, 128)?;
        Ok(())
    }
}
