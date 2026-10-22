//! A path id that is not an integer answers the documented `400 text/plain` on every operation
//! (MAIR-262): the routes extracting their id with `web::Path` answered actix's `404`, those behind
//! `access_guard_middleware` a `400`, while the contract documents `400`.
//!
//! Driven by the published contract (`ApiDoc`), so a new operation with a path id is covered as
//! soon as it is documented.

use crate::common::core_db_url;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::{http::Method, http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::swagger::ApiDoc;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;
use serial_test::serial;
use utoipa::OpenApi;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Every published operation with a path parameter, as `(method, uri, documented statuses)`,
/// each path parameter replaced by `abc`.
fn operations_with_a_path_id() -> Vec<(Method, String, Vec<String>)> {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let mut operations = Vec::new();
    for (template, methods) in document["paths"].as_object().unwrap() {
        if !template.contains('{') {
            continue;
        }
        let uri = template
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') {
                    "abc"
                } else {
                    segment
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        for (method, operation) in methods.as_object().unwrap() {
            let statuses = operation["responses"]
                .as_object()
                .map(|responses| responses.keys().cloned().collect())
                .unwrap_or_default();
            operations.push((
                Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),
                uri.clone(),
                statuses,
            ));
        }
    }
    operations
}

#[tokio::test]
#[serial]
async fn a_path_id_that_is_not_an_integer_answers_the_documented_400() {
    std::sync::LazyLock::force(&INIT);
    let (_container, _host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), core_db_url().await).await);
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(public_config)
            .service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
    )
    .await;
    let admin = generate_jwt(&ADMIN_ID.get().unwrap().to_string(), "admin").unwrap();

    let operations = operations_with_a_path_id();
    assert!(operations.len() >= 15, "{operations:?}");
    let mut failures = Vec::new();
    for (method, uri, documented) in operations {
        if !documented.iter().any(|status| status == "400") {
            failures.push(format!("{method} {uri}: 400 is not documented"));
        }
        // A valid body, so that only the path can be refused.
        let req = test::TestRequest::default()
            .method(method.clone())
            .uri(&format!("{uri}?ressource_type=groups"))
            .insert_header(("Authorization", format!("Bearer {admin}")))
            .set_json(json!({ "name": "abc", "description": "abc", "new_password": "Path!Ids123", "role_id": 1, "user_id": 1 }))
            .to_request();
        let (status, content_type, body) = match test::try_call_service(&app, req).await {
            Ok(resp) => {
                let content_type = resp.headers().get(CONTENT_TYPE).cloned();
                (
                    resp.status(),
                    content_type,
                    String::from_utf8_lossy(&test::read_body(resp).await).to_string(),
                )
            }
            Err(err) => {
                let resp = err.error_response();
                let content_type = resp.headers().get(CONTENT_TYPE).cloned();
                let body = actix_web::body::to_bytes(resp.into_body())
                    .await
                    .unwrap_or_default();
                (
                    err.as_response_error().status_code(),
                    content_type,
                    String::from_utf8_lossy(&body).to_string(),
                )
            }
        };
        let content_type = content_type
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        if status != StatusCode::BAD_REQUEST || !content_type.starts_with("text/plain") {
            failures.push(format!("{method} {uri}: {status} {content_type} {body}"));
        } else if body.contains("abc") {
            failures.push(format!("{method} {uri}: the value is echoed: {body}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
