mod delete;
pub mod doc;
mod get;
mod users;

use actix_web::middleware::from_fn;
use actix_web::web;
use mairie360_api_lib::security::access_guard_middleware;
use mairie360_api_lib::security::AccessCheckConfig;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{group_id}")
            // 1. Les routes standards
            .service(get::endpoint::get)
            .configure(users::config)
            // 2. On applique le middleware et la config UNIQUEMENT au delete
            // en l'enveloppant dans un scope vide ""
            .service(
                web::scope("")
                    .app_data(AccessCheckConfig {
                        resource_name: "groups",
                        action: "update",
                        id_param_pattern: Some("group_id"),
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .service(delete::endpoint::delete), // On réutilise le service existant avec sa macro
            ),
    );
}
