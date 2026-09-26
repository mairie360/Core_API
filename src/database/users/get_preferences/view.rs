use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Display preferences of a user. Every field is nullable: `null` means "use the application
/// default", and a user who never saved a preference gets every field `null`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct UserPreferences {
    /// `light`, `dark` or `system`.
    #[schema(example = "dark")]
    pub theme: Option<String>,
    #[schema(example = "Marianne")]
    pub font_family: Option<String>,
    /// Font size in pixels.
    #[schema(example = 16)]
    pub font_size: Option<i16>,
    #[schema(example = "compact")]
    pub density: Option<String>,
    #[schema(example = "fr")]
    pub language: Option<String>,
    #[schema(example = "Europe/Paris")]
    pub timezone: Option<String>,
    #[schema(example = "DD/MM/YYYY")]
    pub date_format: Option<String>,
    /// Page opened after sign-in.
    #[schema(example = "/dashboard")]
    pub home_page: Option<String>,
    /// Open the notification panel automatically when a notification arrives.
    #[schema(example = false)]
    pub auto_open_notifications: Option<bool>,
}

/// Reads the preferences of a user: always one row, every field `null` when none is saved.
#[derive(Debug, Deserialize)]
pub struct GetPreferencesQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetPreferencesQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }
}

impl ApiRequestDto for GetPreferencesQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(t) FROM ( \
         SELECT p.theme, p.font_family, p.font_size, p.density, p.language, p.timezone, p.date_format, p.home_page, p.auto_open_notifications \
         FROM (SELECT $1::int AS user_id) u \
         LEFT JOIN user_preferences p ON p.user_id = u.user_id \
         ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetPreferencesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GetPreferencesQueryView: user_id = {}", self.user_id)
    }
}
