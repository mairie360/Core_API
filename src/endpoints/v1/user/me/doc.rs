use crate::endpoints::v1::user::me::get::endpoint::__path_get_me;
use crate::endpoints::v1::user::me::notifications::endpoint::{
    __path_get_my_notification_settings, __path_patch_my_notification_settings,
};
use crate::endpoints::v1::user::me::patch::endpoint::__path_patch_me;
use crate::endpoints::v1::user::me::preferences::endpoint::{
    __path_get_my_preferences, __path_patch_my_preferences,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_me,
        patch_me,
        get_my_preferences,
        patch_my_preferences,
        get_my_notification_settings,
        patch_my_notification_settings
    ),
    components(schemas(
        super::get::view::GetMeResponseView,
        super::patch::view::PatchMeView,
        super::preferences::view::PatchPreferencesView,
        super::notifications::view::PatchNotificationSettingsView,
        crate::database::users::get_preferences::UserPreferences,
        crate::database::users::get_notification_settings::UserNotificationSettings
    ))
)]
pub struct MeDoc;
