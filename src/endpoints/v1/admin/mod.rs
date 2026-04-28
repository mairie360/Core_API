pub mod doc;
pub mod roles;
pub mod sessions;
pub mod users;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .configure(roles::config)
            .configure(sessions::config)
            .configure(users::config),
    );
}
