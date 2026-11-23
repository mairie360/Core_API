pub mod admin_guard;
pub mod db_error;
pub mod export_data;
pub mod headers;
pub mod health;
pub mod last_admin;
pub mod pagination;
pub mod password_identity;
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

/// Routes under `/api` reachable without a valid JWT, to register before the `/api` scope.
pub fn public_config(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1::sessions::public_config);
}
