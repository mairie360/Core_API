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

/// Message of the `400` answered for a path parameter that does not parse. It names the rule,
/// never the value received (MAIR-290).
const INVALID_PATH_MESSAGE: &str = "Invalid path: ids are integers.";

/// A path id that is not an integer answers `400 text/plain`, as the contract documents, instead
/// of actix's default `404` (MAIR-262). The routes behind `access_guard_middleware` already
/// answered `400`, the others did not.
fn path_config() -> web::PathConfig {
    web::PathConfig::default()
        .error_handler(|_, _| actix_web::error::ErrorBadRequest(INVALID_PATH_MESSAGE))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.app_data(path_config()).configure(v1::config);
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
