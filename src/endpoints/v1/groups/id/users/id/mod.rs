mod delete;
pub mod doc;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/{user_id}").service(delete::endpoint::delete));
}
