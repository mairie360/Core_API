pub mod doc;
mod roles;

use actix_web::web;
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/users/{userId}").configure(roles::config));
}
