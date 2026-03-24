pub mod auth;
pub mod doc;
pub mod user;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(auth::config)
            .configure(user::config),
    );
}
