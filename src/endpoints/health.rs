use actix_web::{get, HttpResponse, Responder};
use utoipa::OpenApi;

/** * Handles a GET request to the /health endpoint.
 * Responds with a simple "OK" message to indicate the service is healthy.
 */
#[utoipa::path(
    get,
    path = "health",
    summary = "Liveness probe",
    description = "Answers `OK` as soon as the process accepts connections. Unauthenticated, used \
                   as the Kubernetes liveness probe. It checks neither Postgres nor Redis: use \
                   `GET /ready` to know whether the replica can serve requests.",
    responses(
        (
            status = 200,
            description = "The process accepts connections.",
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
