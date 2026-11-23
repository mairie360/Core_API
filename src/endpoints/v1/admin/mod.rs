pub mod doc;
pub mod roles;
pub mod services;
pub mod two_factor_policy;
pub mod users;

use crate::endpoints::admin_guard::admin_guard;
use actix_web::middleware::from_fn;
use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            // On the scope itself, not decided from the path: see `admin_guard` (MAIR-390).
            .wrap(from_fn(admin_guard))
            .configure(roles::config)
            .configure(services::config)
            .configure(two_factor_policy::config)
            .configure(users::config),
    );
}
