pub mod admin;
pub mod auth;
pub mod doc;
pub mod groups;
pub mod ressources;
pub mod roles;
pub mod sessions;
pub mod user;

use actix_web::web;
use mairie360_api_lib::security::AdminMiddleware;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(auth::config)
            .configure(groups::config)
            .configure(roles::config)
            .configure(sessions::config)
            .configure(user::config)
            .wrap(AdminMiddleware)
            .configure(admin::config),
    );
}
