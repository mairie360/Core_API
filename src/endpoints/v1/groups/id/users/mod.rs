pub mod doc;
mod get;
mod id;
mod post;

use actix_web::middleware::from_fn;
use actix_web::web;
use mairie360_api_lib::security::{access_guard_middleware, AccessCheckConfig};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .configure(id::config)
            .service(get::endpoint::get)
            .service(
                web::scope("")
                    .app_data(AccessCheckConfig {
                        resource_name: "groups",
                        action: "update",
                        id_param_pattern: Some("group_id"),
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .service(post::endpoint::post), // On réutilise le service existant avec sa macro
            ),
    );
}
