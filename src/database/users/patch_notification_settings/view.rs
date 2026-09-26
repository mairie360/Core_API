use crate::database::users::nullable_patch::push_bool;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use std::fmt::Display;

/// Fields of a notification settings update. Outer `None`: keep the stored value; `Some(None)`:
/// reset it to the default (`NULL`); `Some(Some(v))`: store `v`.
#[allow(clippy::option_option)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NotificationSettingsPatch {
    pub email: Option<Option<bool>>,
    pub push: Option<Option<bool>>,
    pub desktop: Option<Option<bool>>,
    pub messages: Option<Option<bool>>,
    pub projects: Option<Option<bool>>,
    pub calendar: Option<Option<bool>>,
}

/// Creates or partially updates the notification settings row of a user and returns the stored
/// state.
#[derive(Debug, serde::Deserialize)]
pub struct PatchNotificationSettingsQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl PatchNotificationSettingsQueryView {
    #[must_use]
    pub fn new(user_id: u64, patch: NotificationSettingsPatch) -> Self {
        let mut params = vec![QueryParam::I32(user_id as i32)];
        for field in [
            patch.email,
            patch.push,
            patch.desktop,
            patch.messages,
            patch.projects,
            patch.calendar,
        ] {
            push_bool(&mut params, field);
        }
        Self { user_id, params }
    }
}

impl ApiRequestDto for PatchNotificationSettingsQueryView {
    fn query_sql(&self) -> &'static str {
        "WITH t AS ( \
         INSERT INTO user_notification_settings (user_id, email, push, desktop, messages, projects, calendar) \
         VALUES ($1, CASE WHEN $2 AND NOT $3 THEN $4::boolean END, CASE WHEN $5 AND NOT $6 THEN $7::boolean END, CASE WHEN $8 AND NOT $9 THEN $10::boolean END, CASE WHEN $11 AND NOT $12 THEN $13::boolean END, CASE WHEN $14 AND NOT $15 THEN $16::boolean END, CASE WHEN $17 AND NOT $18 THEN $19::boolean END) \
         ON CONFLICT (user_id) DO UPDATE SET \
         email = CASE WHEN $2 THEN EXCLUDED.email ELSE user_notification_settings.email END, push = CASE WHEN $5 THEN EXCLUDED.push ELSE user_notification_settings.push END, desktop = CASE WHEN $8 THEN EXCLUDED.desktop ELSE user_notification_settings.desktop END, messages = CASE WHEN $11 THEN EXCLUDED.messages ELSE user_notification_settings.messages END, projects = CASE WHEN $14 THEN EXCLUDED.projects ELSE user_notification_settings.projects END, calendar = CASE WHEN $17 THEN EXCLUDED.calendar ELSE user_notification_settings.calendar END \
         RETURNING email, push, desktop, messages, projects, calendar \
         ) \
         SELECT to_jsonb(t) FROM t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for PatchNotificationSettingsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PatchNotificationSettingsQueryView: user_id = {}",
            self.user_id
        )
    }
}
