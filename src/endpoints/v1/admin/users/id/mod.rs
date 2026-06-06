mod delete;
pub mod doc;
mod get;
mod patch;
mod roles;

use actix_web::web;
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{userId}")
            .configure(roles::config)
            .service(delete::endpoint::admin_delete_user)
            .service(patch::endpoint::admin_patch_user)
            .service(get::endpoint::admin_get_user),
    );
}
