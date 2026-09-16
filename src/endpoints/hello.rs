use actix_web::{post, HttpResponse, Responder};
use utoipa::OpenApi;

#[utoipa::path(
    post,
    path = "",
    summary = "Message de bienvenue",
    description = "Renvoie un message fixe. Route non authentifiée, héritée du gabarit d'API et \
                   conservée comme sonde de bout en bout : elle ne porte aucune logique métier.",
    responses(
        (
            status = 200,
            description = "Message de bienvenue.",
            body = String,
            content_type = "text/plain",
            example = json!("Hello, world!")
        )
    ),
    tag = "Service"
)]
#[post("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

#[derive(OpenApi)]
#[openapi(paths(hello,))]
pub struct HelloDoc;
