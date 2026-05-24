pub mod doc;
mod id;
mod post;

use actix_web::web;
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users/{userId}")
            .configure(id::config)
            .service(post::endpoint::post),
    );
}
