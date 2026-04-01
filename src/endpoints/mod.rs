pub mod health;
pub mod hello;
pub mod swagger;
pub mod v1;

use actix_web::{web, HttpMessage};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.configure(v1::config);
    cfg.service(health::health);
    cfg.service(hello::hello);
}

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures_util::future::{ready, Ready};
// Importe ici tes Claims ou ta logique de décodage

pub struct AuthenticatedUser {
    pub id: u64,
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Comme ton Middleware a DEJA validé le token et l'a mis dans les extensions :
        if let Some(user) = req.extensions().get::<AuthenticatedUser>() {
            return ready(Ok(AuthenticatedUser { id: user.id }));
        }

        // Si on arrive ici, c'est que le middleware n'a pas fait son job
        // ou que la route n'est pas protégée
        ready(Err(actix_web::error::ErrorUnauthorized(
            "User not authenticated",
        )))
    }
}
