use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{get, HttpResponse, Responder};
use mairie360_api_lib::jwt_manager::session_jwks;

#[utoipa::path(
    get,
    path = "",
    summary = "Public keys of the session JWTs (JWKS)",
    description = "Lists the Ed25519 public keys session JWTs are verified with (MAIR-680), as a \
                   JSON Web Key Set: the keys of `JWT_PUBLIC_KEY`, never the private key. Only \
                   Core signs session JWTs (`alg` `EdDSA`, `kid` = RFC 7638 thumbprint of the \
                   key, `iss` `mairie360-core`, `aud` `mairie360`); every API and BFF verifies \
                   them with these keys, which they receive through their own `JWT_PUBLIC_KEY` \
                   (this route is informational: nothing fetches it at runtime).\n\n\
                   During a key rotation the set holds two keys. It is empty while the instance \
                   still signs with the historical shared `JWT_SECRET` (HS256). Public, no JWT \
                   needed; cacheable for 5 minutes.",
    responses(
        (
            status = 200,
            description = "The published public keys.",
            body = super::view::JwksView,
            example = json!({
                "keys": [{
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "x": "KW8bV4kiFUo8hmOEU3-lXdqZL1wmKvBw6smUyjpmXks",
                    "kid": "636czspyshqVvkoHmLoy2qx0PD-QuW3R2a1XnN5peL0",
                    "alg": "EdDSA",
                    "use": "sig"
                }]
            })
        ),
        (
            status = 500,
            description = "`JWT_PUBLIC_KEY` is malformed (the API refuses to start in that case, so this only happens if it was changed under a running process).",
            body = String,
            content_type = "text/plain",
            example = json!("The session keys are not configured correctly.")
        )
    ),
    tag = "Auth"
)]
#[get("/jwks")]
pub async fn jwks() -> impl Responder {
    match session_jwks() {
        Ok(jwks) => HttpResponse::Ok()
            .insert_header(CacheControl(vec![
                CacheDirective::Public,
                CacheDirective::MaxAge(300),
            ]))
            .json(jwks),
        Err(error) => {
            tracing::error!(error = %error, "Session public keys cannot be read");
            HttpResponse::InternalServerError()
                .body("The session keys are not configured correctly.")
        }
    }
}
