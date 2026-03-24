pub mod doc;
pub mod login;
pub mod register;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(login::endpoint::login)
            .service(register::endpoint::register),
    );
}
