//! Writes that span several statements run in one transaction (MAIR-420): when a later statement
//! fails, the earlier ones are rolled back.

use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;
use serial_test::serial;
use sqlx::PgPool;

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

fn admin_bearer() -> (&'static str, String) {
    let token = generate_jwt(&ADMIN_ID.get().unwrap().to_string(), "admin").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

async fn setup() -> (web::Data<AppState>, PgPool, i32, i32) {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let pool = get_pool(host.clone()).await;
    let first = create_user(&pool, "First", &unique_marker("tx")).await;
    let second = create_user(&pool, "Second", &unique_marker("tx")).await;
    (state, get_raw_pool(host.clone()).await, first, second)
}

async fn open_session(raw: &PgPool, user_id: i32) {
    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, device_info, ip_address, expires_at) \
         VALUES ($1, $2, 'tx-test', '127.0.0.1', NOW() + INTERVAL '1 hour')",
    )
    .bind(user_id)
    .bind(format!("tx-{}", uuid::Uuid::new_v4()))
    .execute(raw)
    .await
    .unwrap();
}

async fn active_sessions(raw: &PgPool, user_id: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .fetch_one(raw)
        .await
        .unwrap()
}

async fn password_of(raw: &PgPool, user_id: i32) -> Option<String> {
    sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(raw)
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
async fn refused_admin_patch_keeps_the_password_and_the_sessions() {
    let (state, raw, user, other) = setup().await;
    let app = init_app!(state);
    open_session(&raw, user).await;
    let password_before = password_of(&raw, user).await;
    let taken_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(other)
        .fetch_one(&raw)
        .await
        .unwrap();

    // The e-mail is taken: the update fails, so the session revocation is rolled back with it.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user}/"))
        .insert_header(admin_bearer())
        .set_json(json!({ "email": taken_email, "password": "Another!Pass123" }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(password_of(&raw, user).await, password_before);
    assert_eq!(active_sessions(&raw, user).await, 1);

    // Accepted: the password and the revocation land together.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user}/"))
        .insert_header(admin_bearer())
        .set_json(json!({ "password": "Another!Pass123" }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    assert_ne!(password_of(&raw, user).await, password_before);
    assert_eq!(active_sessions(&raw, user).await, 0);
}

#[tokio::test]
#[serial]
async fn role_checks_and_writes_are_one_transaction() {
    let (state, raw, _, _) = setup().await;
    let app = init_app!(state);
    let protected: i32 =
        sqlx::query_scalar("SELECT id FROM roles WHERE can_be_deleted = false ORDER BY id LIMIT 1")
            .fetch_one(&raw)
            .await
            .unwrap();
    let name = format!("tx-role-{}", uuid::Uuid::new_v4().simple());
    let deletable: i32 = sqlx::query_scalar(
        "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, 'tx', true) \
         RETURNING id",
    )
    .bind(&name)
    .fetch_one(&raw)
    .await
    .unwrap();

    let delete = |id: i32| {
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/roles/{id}"))
            .insert_header(admin_bearer())
            .to_request()
    };
    assert_eq!(
        test::call_service(&app, delete(protected)).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        test::call_service(&app, delete(deletable)).await.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        test::call_service(&app, delete(deletable)).await.status(),
        StatusCode::NOT_FOUND
    );

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/roles/{deletable}"))
        .insert_header(admin_bearer())
        .set_json(json!({ "description": "gone" }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
#[serial]
async fn membership_removal_checks_and_removes_in_one_statement() {
    let (state, raw, owner, member) = setup().await;
    let app = init_app!(state);
    let group: i32 = sqlx::query_scalar(
        "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, 'tx') RETURNING id",
    )
    .bind(owner)
    .bind(format!("tx-{}", uuid::Uuid::new_v4()))
    .fetch_one(&raw)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO group_members (group_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(group)
    .bind(member)
    .execute(&raw)
    .await
    .unwrap();

    let remove = || {
        let token = generate_jwt(&owner.to_string(), "user").unwrap();
        test::TestRequest::delete()
            .uri(&format!("/api/v1/groups/{group}/users/{member}/"))
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request()
    };
    assert!(test::call_service(&app, remove())
        .await
        .status()
        .is_success());
    assert_eq!(
        test::call_service(&app, remove()).await.status(),
        StatusCode::NOT_FOUND
    );
}
