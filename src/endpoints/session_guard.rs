//! Rejects the JWT of a revoked or expired session (MAIR-226).
//!
//! Runs inside the lib's `JwtMiddleware` on the `/api` scope. `JwtMiddleware` only checks the
//! signature, the expiry and that the user exists; this guard also checks the session the JWT is
//! bound to (`sid` claim, see [`crate::session_jwt`]), so a JWT stops working as soon as its
//! session is revoked (logout, `sessions/revoke`, admin revocation, archived account).
//!
//! JWTs without `sid` (issued before MAIR-226) are let through: they expire after `JWT_TIMEOUT`.

use crate::database::sessions::is_session_active::IsSessionActiveQueryView;
use crate::session_jwt::decode_session_jwt;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{web, Error};
use mairie360_api_lib::jwt_manager::get_jwt_from_request;
use mairie360_api_lib::state::AppState;

/// Full path of `POST /api/v1/sessions/logout`.
pub const LOGOUT_PATH: &str = "/api/v1/sessions/logout";

/// Use with `actix_web::middleware::from_fn(session_guard)`, wrapped **inside** `JwtMiddleware`.
///
/// # Errors
///
/// `401` when the JWT's session is revoked or expired, `500` when it cannot be checked.
// actix-web runs each worker on a single-threaded runtime: middleware futures need not be Send.
#[allow(clippy::future_not_send)]
pub async fn session_guard(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // Same public paths as `JwtMiddleware`: a stale header must not break login & co. Logout is
    // let through too, so that logging out twice answers 204 instead of 401 (idempotent).
    let skip = req.path().contains("/auth") || req.path() == LOGOUT_PATH;
    let bound_session = if skip {
        None
    } else {
        get_jwt_from_request(req.request())
            .and_then(|jwt| decode_session_jwt(&jwt).ok())
            .and_then(|claims| Some((claims.session_id()?, claims.user_id()?)))
    };

    if let Some((session_id, user_id)) = bound_session {
        let state = req
            .app_data::<web::Data<AppState>>()
            .cloned()
            .ok_or_else(|| {
                actix_web::error::ErrorInternalServerError("Missing application state.")
            })?;
        let active: bool = state
            .get_smart_db()
            .fetch_scalar(&IsSessionActiveQueryView::new(session_id, user_id))
            .await
            .map_err(|_| {
                actix_web::error::ErrorInternalServerError(
                    "An error occurred while accessing the database.",
                )
            })?;
        if !active {
            return Err(actix_web::error::ErrorUnauthorized(
                "Unauthorized: session revoked or expired.",
            ));
        }
    }

    next.call(req).await
}
