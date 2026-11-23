mod delete;
pub mod doc;
mod erase;
mod export;
mod get;
mod password;
mod patch;
mod roles;
mod service;
mod sessions;
mod suspension;
mod two_factor;

use actix_web::web;
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{userId}")
            .configure(roles::config)
            .service(delete::endpoint::admin_delete_user)
            .service(erase::endpoint::admin_erase_user)
            .service(export::endpoint::admin_export_user_data)
            .service(patch::endpoint::admin_patch_user)
            .service(password::endpoint::admin_reset_user_password)
            .service(two_factor::endpoint::admin_reset_user_two_factor)
            .service(sessions::endpoint::admin_revoke_user_sessions)
            .service(sessions::endpoint::admin_revoke_user_session)
            .service(service::endpoint::admin_set_user_service)
            .service(suspension::endpoint::admin_suspend_user)
            .service(suspension::endpoint::admin_reactivate_user)
            .service(get::endpoint::admin_get_user),
    );
}
