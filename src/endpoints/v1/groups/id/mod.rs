mod delete;
pub mod doc;
mod get;
mod patch;
pub mod read_access;
mod users;

use actix_web::middleware::from_fn;
use actix_web::web;
use mairie360_api_lib::security::access_guard_middleware;
use mairie360_api_lib::security::AccessCheckConfig;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{group_id}")
            // 1. The standard routes
            .service(get::endpoint::get_group)
            .configure(users::config)
            // 2. The middleware and the config apply ONLY to delete and patch,
            // wrapped in an empty scope ""
            .service(
                web::scope("")
                    .app_data(AccessCheckConfig {
                        resource_name: "groups",
                        action: "update",
                        id_param_pattern: Some("group_id"),
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .service(delete::endpoint::delete_group) // Reuses the existing service with its macro
                    .service(patch::endpoint::patch_group),
            ),
    );
}
