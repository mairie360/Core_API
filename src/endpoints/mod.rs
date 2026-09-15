pub mod health;
pub mod hello;
pub mod swagger;
pub mod v1;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1::config);
    cfg.service(health::health);
    cfg.service(hello::hello);
}

/// Routes sous `/api` accessibles sans JWT valide, à enregistrer avant le scope `/api`.
pub fn public_config(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1::sessions::public_config);
}
