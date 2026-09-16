use actix_web::{get, HttpResponse, Responder};
use utoipa::OpenApi;

/** * Handles a GET request to the /health endpoint.
 * Responds with a simple "OK" message to indicate the service is healthy.
 */
#[utoipa::path(
    get,
    path = "health",
    summary = "Sonde de vivacité",
    description = "Répond `OK` dès que le processus accepte des connexions. Route non \
                   authentifiée, utilisée comme healthcheck par Docker et Kubernetes. Elle ne \
                   vérifie ni la base de données ni Redis : un `200` ne garantit donc pas que les \
                   dépendances du service soient joignables.",
    responses(
        (
            status = 200,
            description = "Le service accepte des connexions.",
            body = String,
            content_type = "text/plain",
            example = json!("OK")
        )
    ),
    tag = "Service"
)]
#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[derive(OpenApi)]
#[openapi(paths(health,))]
pub struct HealthDoc;
