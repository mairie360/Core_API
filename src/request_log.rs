//! Request logging without personal data (MAIR-290).
//!
//! The root span of `TracingLogger` records the path **and the query string** (`http.target`)
//! and the `Debug` of the error (`exception.details`), which can hold the values a request
//! received. `mairie360_api_lib::request_log` (feature `tracing-actix`) removes both:
//! [`hide_query`] / [`restore_query`] keep the query out of the root span and
//! [`RedactedRootSpanBuilder`] records [`describe_error`] instead. `main` wires them; the tests
//! below pin the behaviour Core relies on.
//!
//! Rule: a log describes an error by its type and context, never by the value it received.

pub use mairie360_api_lib::request_log::{
    describe_error, hide_query, restore_query, RedactedRootSpanBuilder,
};

#[cfg(test)]
mod tests {
    use super::{describe_error, hide_query, restore_query, RedactedRootSpanBuilder};
    use actix_web::test as actix_test;
    use actix_web::{get, middleware, post, web, App, HttpRequest, HttpResponse};
    use mairie360_api_lib::logging::telemetry::TracingLogger;
    use serde::Deserialize;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    /// Collects what the `fmt` subscriber writes, to search it like the marker test does.
    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    impl Write for Captured {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Captured {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    #[derive(Deserialize)]
    struct Body {
        #[allow(dead_code)]
        user_id: i32,
    }

    #[get("/search")]
    #[allow(clippy::future_not_send)]
    async fn search(req: HttpRequest) -> HttpResponse {
        tracing::warn!("searching");
        HttpResponse::BadRequest().body(req.query_string().to_string())
    }

    #[post("/typed")]
    async fn typed(_: web::Json<Body>) -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn the_logs_hold_neither_the_query_nor_the_refused_value() {
        let captured = Captured::default();
        let writer = captured.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let app = actix_test::init_service(
            App::new()
                .wrap(middleware::from_fn(restore_query))
                .wrap(TracingLogger::<RedactedRootSpanBuilder>::new())
                .wrap(middleware::from_fn(hide_query))
                .service(search)
                .service(typed),
        )
        .await;

        let req = actix_test::TestRequest::get()
            .uri("/search?search=gdpr.marker%40example.com&page=2")
            .to_request();
        let body = actix_test::call_and_read_body(&app, req).await;
        // The handler still reads the query.
        assert_eq!(body, "search=gdpr.marker%40example.com&page=2");

        let req = actix_test::TestRequest::post()
            .uri("/typed")
            .set_json(serde_json::json!({ "user_id": "markertracervalue" }))
            .to_request();
        let response = actix_test::call_service(&app, req).await;
        assert_eq!(response.status(), 400);

        let logs = captured.text();
        assert!(logs.contains("http.target=/search"), "{logs}");
        assert!(logs.contains("searching"), "{logs}");
        assert!(logs.contains("Json deserialize error"), "{logs}");
        assert!(!logs.contains("gdpr.marker"), "{logs}");
        assert!(!logs.contains("page=2"), "{logs}");
        assert!(!logs.contains("markertracervalue"), "{logs}");
    }

    #[test]
    fn extractor_errors_lose_their_values() {
        let json: serde_json::Error = serde_json::from_str::<Body>(r#"{"user_id": "secret"}"#)
            .err()
            .unwrap();
        let error = actix_web::Error::from(actix_web::error::JsonPayloadError::Deserialize(json));
        let text = describe_error(&error);
        assert!(
            text.starts_with("Json deserialize error: invalid type: string"),
            "{text}"
        );
        assert!(!text.contains("secret"), "{text}");
        let error = actix_web::error::ErrorBadRequest("Invalid `email`: too long");
        assert_eq!(describe_error(&error), "Invalid `email`: too long");
    }
}
