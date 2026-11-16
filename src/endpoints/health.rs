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
    HttpResponse::Ok().content_type("text/plain").body("OK")
}

#[derive(OpenApi)]
#[openapi(paths(health,))]
pub struct HealthDoc;

#[cfg(test)]
mod tests {
    use super::health;
    use actix_web::{http::header::CONTENT_TYPE, test, App};

    /// MAIR-262: the answer carries the `text/plain` the spec documents (it had no content type).
    #[actix_web::test]
    async fn health_answers_ok_as_text_plain() {
        let app = test::init_service(App::new().service(health)).await;
        let resp =
            test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;
        assert!(resp.status().is_success());
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "text/plain");
        assert_eq!(test::read_body(resp).await, "OK");
    }
}
