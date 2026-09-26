use mairie360_api_lib::database::db_interface::{ApiRequestDto, QueryParam};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

/// Notification settings of a user, per channel and per module. Every field is nullable: `null`
/// means "use the application default", and a user who never saved a setting gets every field
/// `null`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct UserNotificationSettings {
    #[schema(example = true)]
    pub email: Option<bool>,
    #[schema(example = false)]
    pub push: Option<bool>,
    #[schema(example = true)]
    pub desktop: Option<bool>,
    #[schema(example = true)]
    pub messages: Option<bool>,
    #[schema(example = true)]
    pub projects: Option<bool>,
    #[schema(example = false)]
    pub calendar: Option<bool>,
}

/// Reads the notification settings of a user: always one row, every field `null` when none is
/// saved.
#[derive(Debug, Deserialize)]
pub struct GetNotificationSettingsQueryView {
    user_id: u64,
    params: Vec<QueryParam>,
}

impl GetNotificationSettingsQueryView {
    #[must_use]
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id as i32)],
        }
    }
}

impl ApiRequestDto for GetNotificationSettingsQueryView {
    fn query_sql(&self) -> &'static str {
        "SELECT to_jsonb(t) FROM ( \
         SELECT p.email, p.push, p.desktop, p.messages, p.projects, p.calendar \
         FROM (SELECT $1::int AS user_id) u \
         LEFT JOIN user_notification_settings p ON p.user_id = u.user_id \
         ) t"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

impl Display for GetNotificationSettingsQueryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetNotificationSettingsQueryView: user_id = {}",
            self.user_id
        )
    }
}
