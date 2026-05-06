mod add_access;
pub mod doc;
mod remove_access;
pub use add_access::view::AccessType;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ressources")
            .service(add_access::endpoint::add_access)
            .service(remove_access::endpoint::remove_access),
    );
}
