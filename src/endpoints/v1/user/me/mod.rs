use actix_web::web;

pub mod doc;
pub mod get;
pub mod patch;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/me")
            .service(get::endpoint::get)
            .service(patch::endpoint::patch),
    );
}
