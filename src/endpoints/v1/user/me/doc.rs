use crate::endpoints::v1::user::me::get::endpoint::__path_get_me;
use crate::endpoints::v1::user::me::notifications::endpoint::{
    __path_get_my_notification_settings, __path_patch_my_notification_settings,
};
use crate::endpoints::v1::user::me::passkeys::endpoint::{
    __path_delete_passkey, __path_list_passkeys, __path_passkey_registration_options,
    __path_register_passkey,
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
        patch_my_notification_settings,
        passkey_registration_options,
        register_passkey,
        list_passkeys,
        delete_passkey
    ),
    components(schemas(
        super::get::view::GetMeResponseView,
        super::patch::view::PatchMeView,
        super::preferences::view::PatchPreferencesView,
        super::notifications::view::PatchNotificationSettingsView,
        super::passkeys::view::PasskeyRegistrationOptionsResponseView,
        super::passkeys::view::RegisterPasskeyView,
        super::passkeys::view::PasskeyListResponseView,
        crate::database::passkeys::PasskeySummary,
        crate::database::users::get_preferences::UserPreferences,
        crate::database::users::get_notification_settings::UserNotificationSettings
    ))
)]
pub struct MeDoc;
