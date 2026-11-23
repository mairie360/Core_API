pub mod doc;
pub mod endpoint;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/security-alerts").service(endpoint::list_security_alerts));
}
