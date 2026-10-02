pub mod admin_guard;
pub mod db_error;
pub mod health;
pub mod hello;
pub mod ready;
pub mod session_guard;
pub mod swagger;
pub mod v1;
pub mod validation;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1::config);
}

/// Routes sous `/api` accessibles sans JWT valide, à enregistrer avant le scope `/api`.
pub fn public_config(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1::sessions::public_config);
}
