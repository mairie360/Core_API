pub mod doc;
mod get;
mod history;
mod logout;
mod refresh;
mod revoke;
pub mod view;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sessions")
            .service(get::endpoint::get_active_sessions)
            .service(history::endpoint::history)
            .service(logout::endpoint::logout)
            .service(revoke::endpoint::revoke),
    );
}

/// Chemin complet du refresh, monté hors du scope `/api` (voir [`public_config`]).
pub const REFRESH_PATH: &str = "/api/v1/sessions/refresh";

/// Routes de sessions qui ne doivent pas passer par `JwtMiddleware`.
///
/// Le refresh sert précisément quand le JWT est expiré, or le middleware rejette tout JWT expiré
/// sur `/api` (seuls les chemins contenant `/auth` y échappent). Cette configuration doit être
/// enregistrée **avant** le scope `/api` : actix sélectionne la première ressource qui correspond.
pub fn public_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource(REFRESH_PATH).route(web::post().to(refresh::endpoint::refresh)));
}
