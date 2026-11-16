use actix_web::http::header::{CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use actix_web::middleware::DefaultHeaders;

/// Headers added to every response that does not set them itself.
///
/// - `X-Content-Type-Options: nosniff`: every response is JSON or plain text, browsers must not
///   sniff it as HTML.
/// - `Content-Type: text/plain; charset=utf-8`: the error bodies (and the plain-text successes)
///   built with `HttpResponse::…().body(…)` carry no content type, while the OpenAPI contract
///   documents them as `text/plain` (MAIR-262). JSON (`.json(…)`) and SSE answers set their own
///   type, which is kept.
pub fn default_headers() -> DefaultHeaders {
    DefaultHeaders::new()
        .add((X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .add((CONTENT_TYPE, "text/plain; charset=utf-8"))
}

#[cfg(test)]
mod tests {
    use super::default_headers;
    use actix_web::http::header::{CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
    use actix_web::{test, web, App, HttpResponse};

    /// MAIR-262: a plain-text error answers `text/plain`, a JSON answer keeps `application/json`.
    #[actix_web::test]
    async fn plain_text_answers_get_a_content_type_and_json_keeps_its_own() {
        let app = test::init_service(
            App::new()
                .wrap(default_headers())
                .route(
                    "/text",
                    web::get().to(|| async { HttpResponse::Conflict().body("taken") }),
                )
                .route(
                    "/json",
                    web::get().to(|| async { HttpResponse::Ok().json(serde_json::json!({})) }),
                ),
        )
        .await;

        let text =
            test::call_service(&app, test::TestRequest::get().uri("/text").to_request()).await;
        assert_eq!(
            text.headers().get(CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            text.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );

        let json =
            test::call_service(&app, test::TestRequest::get().uri("/json").to_request()).await;
        assert_eq!(
            json.headers().get(CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(
            json.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
    }
}
