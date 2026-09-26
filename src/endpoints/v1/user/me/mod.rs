use actix_web::web;

pub mod doc;
pub mod get;
pub mod notifications;
pub mod nullable;
pub mod patch;
pub mod preferences;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/me")
            .service(get::endpoint::get_me)
            .service(patch::endpoint::patch_me)
            .service(preferences::endpoint::get_my_preferences)
            .service(preferences::endpoint::patch_my_preferences)
            .service(notifications::endpoint::get_my_notification_settings)
            .service(notifications::endpoint::patch_my_notification_settings),
    );
}
