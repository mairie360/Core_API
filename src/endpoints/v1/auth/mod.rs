pub mod doc;
pub mod force_change_password;
pub mod forgot_password;
pub mod jwks;
pub mod keycloak;
pub mod login;
pub mod logout;
pub mod reset_password;

use actix_web::web;
use mairie360_api_lib::smart_db::SmartDatabase;

use crate::database::sessions::create_session::CreateSessionQueryView;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(force_change_password::endpoint::force_change_password)
            .service(forgot_password::endpoint::forgot_password)
            .service(jwks::endpoint::jwks)
            .service(keycloak::endpoint::keycloak_login)
            .service(login::endpoint::login)
            .service(logout::endpoint::logout)
            .service(reset_password::endpoint::reset_password),
    );
}

/// Creates a new session without touching the user's existing sessions.
///
/// Each sign-in (device) keeps its own active session. Sessions only end through
/// expiry, sign-out, explicit revocation or archiving of the account.
///
/// "The previous session of the same device" is not revoked: the IP Core sees is the
/// BFF's and `device_info` is a User-Agent, so two different devices often share the
/// same pair and used to sign each other out.
pub async fn create_new_session(smart_db: &SmartDatabase, view: CreateSessionQueryView) {
    smart_db
        .execute(view)
        .await
        .map_err(|e| {
            tracing::error!("Create Session DB Error: {e}");
        })
        .ok();
}
