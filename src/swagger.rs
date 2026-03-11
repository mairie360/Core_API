use actix_web::{get, post, HttpResponse, Responder};

use crate::endpoints::login::login_view::LoginView;
use crate::endpoints::register::register_view::RegisterView;

use utoipa::OpenApi;

//                                        -- POST REQUESTS --

/** * Handles a POST request to the root endpoint.
 * Responds with a simple "Hello, world!" message.
 */
#[utoipa::path(
    post,
    path = "/",
    responses(
        (status = 200, description = "Returns a greeting message", body = String)
    )
)]
#[post("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

//                                        -- GET REQUESTS --

/** * Handles a GET request to the /health endpoint.
 * Responds with a simple "OK" message to indicate the service is healthy.
 */
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    )
)]
#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
        hello,
        crate::endpoints::login::login_request::login,
        crate::endpoints::register::register_request::register
    ),
    components(
        schemas(LoginView, RegisterView)
    ),
    tags(
        (name = "Core API", description = "Endpoints for core functionalities")
    )
)]
pub struct ApiDoc;
