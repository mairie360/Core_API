pub mod doc;
mod get;
mod history;
mod logout;
mod refresh;
mod report;
mod revoke;
mod revoke_by_id;
mod revoke_others;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sessions")
            .service(get::endpoint::get_active_sessions)
            .service(history::endpoint::history)
            .service(logout::endpoint::logout)
            .service(revoke::endpoint::revoke)
            .service(revoke_others::endpoint::revoke_other_sessions)
            .service(report::endpoint::report_session_use)
            .service(revoke_by_id::endpoint::revoke_session_by_id),
    );
}

/// Full path of the refresh, mounted outside the `/api` scope (see [`public_config`]).
pub const REFRESH_PATH: &str = "/api/v1/sessions/refresh";

/// Session routes that must not go through `JwtMiddleware`.
///
/// The refresh is used precisely when the JWT has expired, and the middleware rejects any expired
/// JWT on `/api` (only the paths holding `/auth` escape it). This configuration must be
/// registered **before** the `/api` scope: actix picks the first matching resource.
pub fn public_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource(REFRESH_PATH).route(web::post().to(refresh::endpoint::refresh)));
}
