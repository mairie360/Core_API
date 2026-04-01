pub mod auth;
pub mod doc;
pub mod sessions;
pub mod users;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(auth::config)
            .configure(sessions::config)
            .configure(users::config),
    );
}
