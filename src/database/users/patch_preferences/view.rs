use crate::database::users::nullable_patch::{push_bool, push_i32, push_text};
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Fields of a preferences update. Outer `None`: keep the stored value; `Some(None)`: reset it to
/// the default (`NULL`); `Some(Some(v))`: store `v`.
#[allow(clippy::option_option)]
#[derive(Debug, Clone, Default)]
pub struct PreferencesPatch<'a> {
    pub theme: Option<Option<&'a str>>,
    pub font_family: Option<Option<&'a str>>,
    pub font_size: Option<Option<i32>>,
    pub density: Option<Option<&'a str>>,
    pub language: Option<Option<&'a str>>,
    pub timezone: Option<Option<&'a str>>,
    pub date_format: Option<Option<&'a str>>,
    pub home_page: Option<Option<&'a str>>,
    pub auto_open_notifications: Option<Option<bool>>,
}

/// Creates or partially updates the preferences row of a user and returns the stored state.
#[derive(Debug, serde::Deserialize)]
pub struct PatchPreferencesQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl PatchPreferencesQueryView {
    #[must_use]
    pub fn new(user_id: u64, patch: &PreferencesPatch<'_>) -> Self {
        let mut params = vec![QueryParam::I32(user_id as i32)];
        push_text(&mut params, patch.theme);
        push_text(&mut params, patch.font_family);
        push_i32(&mut params, patch.font_size);
        push_text(&mut params, patch.density);
        push_text(&mut params, patch.language);
        push_text(&mut params, patch.timezone);
        push_text(&mut params, patch.date_format);
        push_text(&mut params, patch.home_page);
        push_bool(&mut params, patch.auto_open_notifications);
        Self { user_id, params }
    }
}

impl ApiRequestDto for PatchPreferencesQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS ( \
         INSERT INTO user_preferences (user_id, theme, font_family, font_size, density, language, timezone, date_format, home_page, auto_open_notifications) \
         VALUES ($1, CASE WHEN $2 AND NOT $3 THEN $4::varchar END, CASE WHEN $5 AND NOT $6 THEN $7::varchar END, CASE WHEN $8 AND NOT $9 THEN $10::smallint END, CASE WHEN $11 AND NOT $12 THEN $13::varchar END, CASE WHEN $14 AND NOT $15 THEN $16::varchar END, CASE WHEN $17 AND NOT $18 THEN $19::varchar END, CASE WHEN $20 AND NOT $21 THEN $22::varchar END, CASE WHEN $23 AND NOT $24 THEN $25::varchar END, CASE WHEN $26 AND NOT $27 THEN $28::boolean END) \
         ON CONFLICT (user_id) DO UPDATE SET \
         theme = CASE WHEN $2 THEN EXCLUDED.theme ELSE user_preferences.theme END, font_family = CASE WHEN $5 THEN EXCLUDED.font_family ELSE user_preferences.font_family END, font_size = CASE WHEN $8 THEN EXCLUDED.font_size ELSE user_preferences.font_size END, density = CASE WHEN $11 THEN EXCLUDED.density ELSE user_preferences.density END, language = CASE WHEN $14 THEN EXCLUDED.language ELSE user_preferences.language END, timezone = CASE WHEN $17 THEN EXCLUDED.timezone ELSE user_preferences.timezone END, date_format = CASE WHEN $20 THEN EXCLUDED.date_format ELSE user_preferences.date_format END, home_page = CASE WHEN $23 THEN EXCLUDED.home_page ELSE user_preferences.home_page END, auto_open_notifications = CASE WHEN $26 THEN EXCLUDED.auto_open_notifications ELSE user_preferences.auto_open_notifications END \
         RETURNING theme, font_family, font_size, density, language, timezone, date_format, home_page, auto_open_notifications \
         ) \
         SELECT to_jsonb(t) FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchPreferencesQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PatchPreferencesQueryView: user_id = {}", self.user_id)
    }
}
