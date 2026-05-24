mod delete;
pub mod doc;
mod post;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/roles")
            .service(delete::endpoint::delete)
            .service(post::endpoint::post),
    );
}
