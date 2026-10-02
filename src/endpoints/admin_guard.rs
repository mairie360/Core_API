//! Administrator check of the `/api/v1/admin` scope (MAIR-390).
//!
//! The lib's `AdminMiddleware` decides from a regular expression on the raw request path, so a
//! percent-encoded path (`/api/v1/%61dmin/users/`) that actix still routes to an admin handler
//! skipped the check. This guard is wrapped on the `/admin` scope itself: it runs for every route
//! mounted there, whatever the spelling of the path, and checks the caller against the database.
//! On success it stores an [`AdminUser`], which every admin handler takes as an argument: a
//! handler mounted outside the guarded scope by mistake answers `403` instead of serving
//! anyone.

use actix_web::body::MessageBody;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{web, Error, FromRequest, HttpMessage, HttpRequest};
use futures_util::future::{ready, Ready};
use mairie360_api_lib::database::query_views::IsAdminQueryView;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

/// An authenticated caller verified as administrator by [`admin_guard`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminUser {
    pub id: u64,
}

impl FromRequest for AdminUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(
            req.extensions().get::<Self>().copied().ok_or_else(|| {
                actix_web::error::ErrorForbidden("Forbidden: User is not an admin.")
            }),
        )
    }
}

/// Use with `actix_web::middleware::from_fn(admin_guard)` on the `/admin` scope, inside
/// `JwtMiddleware` (which stores the [`AuthenticatedUser`]).
///
/// # Errors
///
/// `401` without an authenticated user, `403` when the user is not an administrator, `500` when
/// the check cannot be made.
// actix-web runs each worker on a single-threaded runtime: middleware futures need not be Send.
#[allow(clippy::future_not_send)]
pub async fn admin_guard(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let user_id = req
        .extensions()
        .get::<AuthenticatedUser>()
        .map(|user| user.id)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("User not authenticated"))?;
    let state = req
        .app_data::<web::Data<AppState>>()
        .cloned()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Missing application state."))?;
    let is_admin: bool = state
        .get_smart_db()
        .fetch_scalar(&IsAdminQueryView::new(user_id))
        .await
        .map_err(|e| {
            tracing::error!("Admin check DB Error: {e}");
            actix_web::error::ErrorInternalServerError(
                "An error occurred while accessing the database.",
            )
        })?;
    if !is_admin {
        return Err(actix_web::error::ErrorForbidden(
            "Forbidden: User is not an admin.",
        ));
    }
    req.extensions_mut().insert(AdminUser { id: user_id });
    next.call(req).await
}
