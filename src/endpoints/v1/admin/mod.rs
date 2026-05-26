pub mod doc;
pub mod roles;
// pub mod sessions;
pub mod users;

use actix_web::web;
// use mairie360_api_lib::security::AdminMiddleware;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            // .wrap(AdminMiddleware)
            .configure(roles::config)
            // .configure(sessions::config)
            .configure(users::config),
    );
}
