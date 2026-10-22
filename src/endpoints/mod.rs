pub mod admin_guard;
pub mod db_error;
pub mod export_data;
pub mod headers;
pub mod health;
pub mod keycloak_console;
pub mod last_admin;
pub mod pagination;
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

/// Root routes of the Keycloak console access (MAIR-609): `GET /admin-access` and
/// `GET /internal/keycloak-console/verify`, outside `/api` and its JWT middleware.
pub fn keycloak_console_config(cfg: &mut web::ServiceConfig) {
    cfg.configure(keycloak_console::config);
}
