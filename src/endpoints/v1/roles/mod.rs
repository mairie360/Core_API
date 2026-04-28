pub mod doc;
mod get;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/roles").service(get::endpoint::get));
}
