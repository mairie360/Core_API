//! Database errors are answered by kind, not with one status per handler (MAIR-421).

use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `sessions_refresh`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .configure(public_config)
                .service(
                    web::scope("/api")
                        .wrap(actix_web::middleware::from_fn(session_guard))
                        .wrap(JwtMiddleware)
                        .configure(config),
                ),
        )
        .await
    };
}

fn bearer(user_id: i32) -> (&'static str, String) {
    let token = generate_jwt(&user_id.to_string(), "user").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

#[tokio::test]
#[serial]
async fn constraint_violations_answer_404_or_409() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    let owner = create_user(&pool, "Owner", &unique_marker("dberr")).await;
    let member = create_user(&pool, "Member", &unique_marker("dberr")).await;
    let group: i32 = sqlx::query_scalar(
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, 'dberr') RETURNING id",
    )
    .bind(owner)
    .bind(format!("dberr-{}", uuid::Uuid::new_v4()))
    .fetch_one(&raw)
    .await
    .unwrap();

    let add = |user_id: i64| {
        test::TestRequest::post()
            .uri(&format!("/api/v1/groups/{group}/users/"))
            .insert_header(bearer(owner))
            .set_json(json!({ "user_id": user_id }))
            .to_request()
    };
    assert_eq!(
        test::call_service(&app, add(member.into())).await.status(),
        StatusCode::OK
    );
    // Already a member: unique constraint.
    assert_eq!(
        test::call_service(&app, add(member.into())).await.status(),
        StatusCode::CONFLICT
    );
    // No such user: foreign key.
    assert_eq!(
        test::call_service(&app, add(i64::from(i32::MAX)))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );

    let role_id: i32 = sqlx::query_scalar("SELECT id FROM roles ORDER BY id LIMIT 1")
        .fetch_one(&raw)
        .await
        .unwrap();
    let grant = || {
        let token = generate_jwt(&ADMIN_ID.get().unwrap().to_string(), "admin").unwrap();
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{member}/roles/"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .set_json(json!({ "role_id": role_id }))
            .to_request()
    };
    assert_eq!(
        test::call_service(&app, grant()).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, grant()).await.status(),
        StatusCode::CONFLICT
    );
}
