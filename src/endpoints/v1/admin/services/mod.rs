//! Services of the mairie (MAIR-923): listed, created, renamed and deleted by administrators.
pub mod doc;
pub mod endpoint;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/services")
            .service(endpoint::admin_list_services)
            .service(endpoint::admin_create_service)
            .service(endpoint::admin_update_service)
            .service(endpoint::admin_delete_service),
    );
}
