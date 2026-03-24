use actix_web::{get, HttpResponse, Responder};
use utoipa::OpenApi;

/** * Handles a GET request to the /health endpoint.
 * Responds with a simple "OK" message to indicate the service is healthy.
 */
#[utoipa::path(
    get,
    path = "health",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    )
)]
#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[derive(OpenApi)]
#[openapi(paths(health,))]
pub struct HealthDoc;
