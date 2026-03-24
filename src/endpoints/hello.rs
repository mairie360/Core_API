use actix_web::{post, HttpResponse, Responder};
use utoipa::OpenApi;

#[utoipa::path(
    post,
    path = "",
    responses(
        (status = 200, description = "Returns a greeting message", body = String)
    )
)]
#[post("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

#[derive(OpenApi)]
#[openapi(paths(hello,))]
pub struct HelloDoc;
