//! Lists that grow with the data are bounded (MAIR-425).

use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::Value;
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

#[tokio::test]
#[serial]
async fn session_history_and_group_members_are_paginated() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
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
    let user = create_user(&pool, "Paged", &unique_marker("page")).await;
    for minutes in 0..5 {
        sqlx::query(
            "INSERT INTO sessions (user_id, token_hash, device_info, ip_address, created_at, \
             expires_at) VALUES ($1, $2, $3, '127.0.0.1', NOW() - make_interval(mins => $4), \
             NOW() + INTERVAL '1 hour')",
        )
        .bind(user)
        .bind(format!("page-{}", uuid::Uuid::new_v4()))
        .bind(format!("device-{minutes}"))
        .bind(minutes)
        .execute(&raw)
        .await
        .unwrap();
    }
    let token = generate_jwt(&user.to_string(), "user").unwrap();
    let get = |uri: &str| {
        test::TestRequest::get()
            .uri(uri)
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request()
    };

    let resp = test::call_service(&app, get("/api/v1/sessions/history?limit=2&offset=1")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let devices: Vec<&str> = body["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|session| session["device_info"].as_str().unwrap())
        .collect();
    assert_eq!(devices, ["device-1", "device-2"], "newest first, offset 1");

    for uri in [
        "/api/v1/sessions/history?limit=0",
        "/api/v1/sessions/history?limit=501",
        "/api/v1/groups/?offset=1000001",
    ] {
        let resp = test::call_service(&app, get(uri)).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{uri}");
    }

    let group: i32 = sqlx::query_scalar(
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, 'page') RETURNING id",
    )
    .bind(user)
    .bind(format!("page-{}", uuid::Uuid::new_v4()))
    .fetch_one(&raw)
    .await
    .unwrap();
    for _ in 0..3 {
        let member = create_user(&pool, "Member", &unique_marker("page")).await;
        sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES ($1, $2)")
            .bind(group)
            .bind(member)
            .execute(&raw)
            .await
            .unwrap();
    }
    let resp =
        test::call_service(&app, get(&format!("/api/v1/groups/{group}/users/?limit=2"))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["users"].as_array().unwrap().len(), 2);
}
