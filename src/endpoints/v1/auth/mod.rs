pub mod doc;
pub mod force_change_password;
pub mod forgot_password;
pub mod login;
pub mod reset_password;

use actix_web::web;
use mairie360_api_lib::smart_db::SmartDatabase;

use crate::database::sessions::create_session::CreateSessionQueryView;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(force_change_password::endpoint::force_change_password)
            .service(forgot_password::endpoint::forgot_password)
            .service(login::endpoint::login)
            .service(reset_password::endpoint::reset_password),
    );
}

/// Crée une nouvelle session sans toucher aux sessions existantes de l'utilisateur.
///
/// Chaque connexion (appareil) garde sa propre session active. Les sessions ne se ferment que
/// par expiration, déconnexion, révocation explicite ou archivage du compte.
///
/// On ne révoque pas « la session précédente du même appareil » : l'IP vue par Core est celle
/// du BFF et `device_info` est un User-Agent, donc deux appareils distincts partagent souvent le
/// même couple et se déconnectaient mutuellement.
pub async fn create_new_session(smart_db: &SmartDatabase, view: CreateSessionQueryView) {
    smart_db
        .execute(view)
        .await
        .map_err(|e| {
            eprintln!("Create Session DB Error: {e}");
        })
        .ok();
}
