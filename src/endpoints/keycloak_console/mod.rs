//! Browser and ingress routes of the Keycloak console access (MAIR-609).
//!
//! Mounted at the root, outside `/api` and its JWT middleware: `GET /admin-access` is reached through the ingress of
//! `auth.<domain>`, `GET /internal/keycloak-console/verify` only by the Traefik `ForwardAuth`
//! middleware, in the cluster. See [`crate::keycloak_console`].

pub mod admin_access;
pub mod verify;

use actix_web::web;
use mairie360_api_lib::database::query_views::IsAdminQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;
use utoipa::OpenApi;

/// Mounts both routes (at the root, like `/health`).
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(admin_access::open_keycloak_console)
        .service(verify::verify_keycloak_console_access);
}

/// Whether `user_id` is still an administrator whose account is not archived (`is_admin()`),
/// read from the database on every call: a demoted or archived administrator loses the console
/// at once.
///
/// # Errors
///
/// The database error, logged here.
pub async fn is_active_admin(state: &AppState, user_id: u64) -> Result<bool, ApiLibError> {
    state
        .get_smart_db()
        .fetch_scalar(&IsAdminQueryView::new(user_id))
        .await
        .inspect_err(|e| {
            tracing::error!("Keycloak console admin check DB Error: {e}");
        })
}

#[derive(OpenApi)]
#[openapi(
    paths(
        admin_access::open_keycloak_console,
        verify::verify_keycloak_console_access
    ),
    components(schemas(admin_access::AdminAccessQuery))
)]
pub struct KeycloakConsoleDoc;
