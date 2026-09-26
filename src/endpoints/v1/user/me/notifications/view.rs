use crate::database::users::patch_notification_settings::NotificationSettingsPatch;
use crate::endpoints::v1::user::me::nullable::double_option;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Partial update of the notification settings. For every field: absent keeps the stored value,
/// `null` resets it to the application default, `true` / `false` stores it. Any other type is
/// refused with `400` by the JSON extractor.
#[allow(clippy::option_option)]
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct PatchNotificationSettingsView {
    /// Notifications by e-mail.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = true)]
    email: Option<Option<bool>>,
    /// Push notifications (mobile).
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = false)]
    push: Option<Option<bool>>,
    /// Desktop notifications (browser).
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = true)]
    desktop: Option<Option<bool>>,
    /// Notifications of the messaging module.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = true)]
    messages: Option<Option<bool>>,
    /// Notifications of the projects module.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = true)]
    projects: Option<Option<bool>>,
    /// Notifications of the calendar module.
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<bool>, example = false)]
    calendar: Option<Option<bool>>,
}

impl PatchNotificationSettingsView {
    #[must_use]
    pub const fn as_patch(&self) -> NotificationSettingsPatch {
        NotificationSettingsPatch {
            email: self.email,
            push: self.push,
            desktop: self.desktop,
            messages: self.messages,
            projects: self.projects,
            calendar: self.calendar,
        }
    }
}
