mod assign;
pub mod doc;
mod remove;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/roles")
            .service(assign::endpoint::assign)
            .service(remove::endpoint::remove),
    );
}
