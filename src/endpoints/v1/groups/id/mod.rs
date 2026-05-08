mod delete;
pub mod doc;
mod get;
mod users;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{group_id}")
            .service(delete::endpoint::delete)
            .service(get::endpoint::get)
            .configure(users::config),
    );
}
