pub mod admin;
pub mod auth;
pub mod doc;
pub mod sessions;
pub mod user;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(admin::config)
            .configure(auth::config)
            .configure(sessions::config)
            .configure(user::config),
    );
}
