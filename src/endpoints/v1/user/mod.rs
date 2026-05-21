pub mod doc;
pub mod id;
pub mod me;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .configure(me::config)
            .configure(id::config),
    );
}
