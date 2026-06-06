mod delete;
pub mod doc;
mod post;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/roles")
            .service(delete::endpoint::admin_delete_user_role)
            .service(post::endpoint::admin_add_role_to_user),
    );
}
