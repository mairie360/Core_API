pub mod doc;
mod get;
mod id;
mod post;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/groups")
            .service(get::endpoint::get)
            .service(post::endpoint::post)
            .configure(id::config),
    );
}
