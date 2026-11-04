pub mod console_access;
pub mod doc;
pub mod migration;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/keycloak")
            .service(migration::endpoint::run_keycloak_migration)
            .service(console_access::endpoint::request_keycloak_console_access),
    );
}
