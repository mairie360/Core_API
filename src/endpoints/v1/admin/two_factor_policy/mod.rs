pub mod doc;
pub mod endpoint;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/two-factor-policy")
            .service(endpoint::get_two_factor_policy)
            .service(endpoint::update_two_factor_policy),
    );
}
