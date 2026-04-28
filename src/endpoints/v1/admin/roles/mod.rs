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
            .service(delete::endpoint::delete)
            .service(get::endpoint::get)
            .service(patch::endpoint::patch)
            .service(post::endpoint::post)
            .service(put::endpoint::put),
    );
}
