pub mod audit;
pub mod doc;
// pub mod get;
pub mod revoke;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sessions")
            .service(audit::endpoint::audit)
            // .service(get::endpoint::get)
            .service(revoke::endpoint::revoke),
    );
}
