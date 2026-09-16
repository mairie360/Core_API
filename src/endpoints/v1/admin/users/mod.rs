pub mod doc;
mod get;
mod id;
mod post;

use actix_web::web;
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .configure(id::config)
            .service(get::endpoint::admin_list_users)
            .service(post::endpoint::admin_post_user),
    );
}
