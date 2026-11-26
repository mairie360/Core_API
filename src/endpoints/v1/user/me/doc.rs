use crate::endpoints::v1::user::me::export::endpoint::__path_export_my_data;
use crate::endpoints::v1::user::me::get::endpoint::__path_get_me;
use crate::endpoints::v1::user::me::notifications::endpoint::{
    __path_get_my_notification_settings, __path_patch_my_notification_settings,
};
use crate::endpoints::v1::user::me::passkeys::endpoint::{
    __path_delete_my_passkey, __path_list_my_passkeys, __path_passkey_register_options,
    __path_register_passkey, __path_rename_my_passkey,
};
use crate::endpoints::v1::user::me::password::endpoint::__path_change_my_password;
use crate::endpoints::v1::user::me::patch::endpoint::__path_patch_me;
use crate::endpoints::v1::user::me::photo::endpoint::{
    __path_delete_my_photo, __path_get_my_photo, __path_set_my_photo,
};
use crate::endpoints::v1::user::me::preferences::endpoint::{
    __path_get_my_preferences, __path_patch_my_preferences,
};
use crate::endpoints::v1::user::me::security_alert::endpoint::{
    __path_acknowledge_my_security_alert, __path_get_my_security_alert,
};
use crate::endpoints::v1::user::me::two_factor::endpoint::{
    __path_confirm_two_factor, __path_disable_two_factor, __path_get_two_factor_status,
    __path_regenerate_backup_codes, __path_setup_two_factor,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        get_me,
        export_my_data,
        patch_me,
        set_my_photo,
        get_my_photo,
        delete_my_photo,
        change_my_password,
        get_my_preferences,
        patch_my_preferences,
        get_my_notification_settings,
        patch_my_notification_settings,
        get_two_factor_status,
        setup_two_factor,
        confirm_two_factor,
        disable_two_factor,
        regenerate_backup_codes,
        list_my_passkeys,
        passkey_register_options,
        register_passkey,
        delete_my_passkey,
        rename_my_passkey,
        get_my_security_alert,
        acknowledge_my_security_alert
    ),
    components(schemas(
        super::get::view::GetMeResponseView,
        super::patch::view::PatchMeView,
        super::password::view::ChangePasswordView,
        super::preferences::view::PatchPreferencesView,
        super::notifications::view::PatchNotificationSettingsView,
        crate::database::users::get_preferences::UserPreferences,
        crate::database::users::get_notification_settings::UserNotificationSettings,
        super::two_factor::view::TwoFactorStatusView,
        super::two_factor::view::TwoFactorSetupView,
        super::two_factor::view::TwoFactorCodeView,
        super::two_factor::view::DisableTwoFactorView,
        super::two_factor::view::BackupCodesView,
        super::passkeys::view::PasskeyView,
        super::passkeys::view::PasskeyRegisterOptionsView,
        super::passkeys::view::PasskeyRegisterView,
        super::passkeys::view::PasskeyRenameView,
        crate::database::security_alerts::SecurityAlert
    ))
)]
pub struct MeDoc;
