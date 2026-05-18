mod delete;
pub mod doc;

use actix_web::{middleware::from_fn, web};
use mairie360_api_lib::security::{access_guard_middleware, AccessCheckConfig};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{user_id}").service(
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
