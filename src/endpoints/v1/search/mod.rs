pub mod doc;
pub mod endpoint;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(endpoint::global_search);
}
