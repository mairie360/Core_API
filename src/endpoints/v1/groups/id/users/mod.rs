pub mod doc;
mod get;
mod id;
mod post;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .configure(id::config)
            .service(post::endpoint::post)
            .service(get::endpoint::get),
    );
}
