use actix_web::web;

pub mod doc;
pub mod export;
pub mod get;
pub mod notifications;
pub mod nullable;
pub mod passkeys;
pub mod password;
pub mod patch;
pub mod preferences;
pub mod two_factor;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/me")
            .service(get::endpoint::get_me)
            .service(export::endpoint::export_my_data)
            .service(patch::endpoint::patch_me)
            .service(password::endpoint::change_my_password)
            .service(preferences::endpoint::get_my_preferences)
            .service(preferences::endpoint::patch_my_preferences)
            .service(notifications::endpoint::get_my_notification_settings)
            .service(notifications::endpoint::patch_my_notification_settings)
            .service(two_factor::endpoint::get_two_factor_status)
            .service(two_factor::endpoint::setup_two_factor)
            .service(two_factor::endpoint::confirm_two_factor)
            .service(two_factor::endpoint::disable_two_factor)
            .service(two_factor::endpoint::regenerate_backup_codes)
            .service(passkeys::endpoint::list_my_passkeys)
            .service(passkeys::endpoint::passkey_register_options)
            .service(passkeys::endpoint::register_passkey)
            .service(passkeys::endpoint::delete_my_passkey),
    );
}
