mod delete;
pub mod doc;
mod get;
mod patch;
mod post;
mod put;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/roles")
            .service(delete::endpoint::admin_delete_role)
            .service(get::endpoint::admin_get_role)
            .service(patch::endpoint::admin_patch_role)
            .service(post::endpoint::admin_post_role)
            .service(put::endpoint::admin_put_role),
    );
}
