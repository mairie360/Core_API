mod delete;
pub mod doc;
mod get;
mod patch;
mod put;
mod roles;

use actix_web::web;
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users/{userId}")
            .configure(roles::config)
            .service(delete::endpoint::delete)
            .service(patch::endpoint::patch)
            .service(get::endpoint::get)
            .service(put::endpoint::put),
    );
}
