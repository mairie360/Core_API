mod admin;
pub mod doc;
mod get;
mod history;
mod refresh;
mod revoke;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sessions")
            .configure(admin::config)
            .service(get::endpoint::get)
            .service(history::endpoint::history)
            .service(refresh::endpoint::refresh)
            .service(revoke::endpoint::revoke),
    );
}
