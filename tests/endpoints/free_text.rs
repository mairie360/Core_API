//! `<` and `>` are ordinary characters in names and descriptions (MAIR-426): the API serves JSON
//! with `nosniff`, escaping for HTML is the fronts' job.

use crate::common::{get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

#[tokio::test]
#[serial]
async fn angle_brackets_round_trip_in_a_group() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
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
    let owner = create_user(&pool, "Owner", &unique_marker("text")).await;
    let token = generate_jwt(&owner.to_string(), "user").unwrap();
    let name = format!("Budget > 10 000 € <3 {}", unique_marker(""));
    let description = "Arbitrages -> <b>2027</b>";

    let req = test::TestRequest::post()
        .uri("/api/v1/groups/")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({ "name": name, "description": description }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let created: Value = test::read_body_json(resp).await;
    let id = created["id"].as_u64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/groups/{id}/"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );
    let group: Value = test::read_body_json(resp).await;
    assert_eq!(group["group"]["name"], json!(name));
    assert_eq!(group["group"]["description"], json!(description));
}
