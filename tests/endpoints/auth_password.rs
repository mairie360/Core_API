use crate::common::{get_pool, get_raw_pool};
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::endpoints::{config, public_config};
use core_api::password::is_hashed_password;
use mairie360_api_lib::{
    security::JwtMiddleware, state::AppState, test_setup::queries_setup::get_shared_db,
};
use serde_json::json;
use serial_test::serial;
use sqlx::Row;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "auth_password_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Même montage que `main.rs` : routes publiques puis scope `/api` protégé par `JwtMiddleware`
/// (qui laisse passer tout ce qui est sous `/auth`).
macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .configure(public_config)
                .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
        )
        .await
    };
}

async fn fetch_stored_password(pool: &sqlx::PgPool, email: &str) -> String {
    sqlx::query("SELECT password FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
        .get("password")
}

/// Insère un utilisateur comme le ferait un compte créé avant cette migration : mot de passe en
/// clair en base, plus de première connexion. `RegisterUserQueryView` n'ajoute plus de hachage
/// (celui-ci est appliqué par l'endpoint), donc elle écrit bien la valeur brute qu'on lui donne.
async fn create_legacy_plaintext_user(
    smart_db: &mairie360_api_lib::smart_db::SmartDatabase,
    raw_pool: &sqlx::PgPool,
    email: &str,
    plaintext_password: &str,
) {
    let _: bool = smart_db
        .fetch_scalar(&RegisterUserQueryView::new(
            "Legacy",
            "User",
            email,
            plaintext_password,
            None,
        ))
        .await
        .unwrap();

    sqlx::query("UPDATE users SET first_connect = false WHERE email = $1")
        .bind(email)
        .execute(raw_pool)
        .await
        .unwrap();
}

#[tokio::test]
#[serial]
async fn register_stores_a_hash_not_the_plaintext_password() {
    let (_container, host) = get_shared_db().await;
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:6379".to_string(), host.clone()).await);
    let app = init_app!(state);
    let raw_pool = get_raw_pool(host.clone()).await;

    let email = format!("hash_register_{}@example.com", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({
            "first_name": "Hash",
            "last_name": "Test",
            "email": email,
            "password": "plaintext_password_123",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let stored_password = fetch_stored_password(&raw_pool, &email).await;
    assert_ne!(stored_password, "plaintext_password_123");
    assert!(is_hashed_password(&stored_password));
}

#[tokio::test]
#[serial]
async fn login_migrates_legacy_plaintext_password_to_a_hash() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:6379".to_string(), host.clone()).await);
    let app = init_app!(state);
    let pool = get_pool(host.clone()).await;
    let raw_pool = get_raw_pool(host.clone()).await;

    let email = format!("legacy_login_{}@example.com", uuid::Uuid::new_v4());
    create_legacy_plaintext_user(&pool, &raw_pool, &email, "legacy_plaintext_pw").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": email,
            "password": "legacy_plaintext_pw",
            "device_info": "test suite",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let stored_password = fetch_stored_password(&raw_pool, &email).await;
    assert_ne!(stored_password, "legacy_plaintext_pw");
    assert!(is_hashed_password(&stored_password));
}

#[tokio::test]
#[serial]
async fn login_rejects_wrong_password_for_legacy_plaintext_account_without_migrating() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:6379".to_string(), host.clone()).await);
    let app = init_app!(state);
    let pool = get_pool(host.clone()).await;
    let raw_pool = get_raw_pool(host.clone()).await;

    let email = format!("legacy_wrong_{}@example.com", uuid::Uuid::new_v4());
    create_legacy_plaintext_user(&pool, &raw_pool, &email, "legacy_plaintext_pw").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": email,
            "password": "wrong_password",
            "device_info": "test suite",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Rejected credentials must never trigger the opportunistic migration.
    let stored_password = fetch_stored_password(&raw_pool, &email).await;
    assert_eq!(stored_password, "legacy_plaintext_pw");
}

#[tokio::test]
#[serial]
async fn login_works_after_a_hashed_login_migration() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:6379".to_string(), host.clone()).await);
    let app = init_app!(state);
    let pool = get_pool(host.clone()).await;
    let raw_pool = get_raw_pool(host.clone()).await;

    let email = format!("legacy_twice_{}@example.com", uuid::Uuid::new_v4());
    create_legacy_plaintext_user(&pool, &raw_pool, &email, "legacy_plaintext_pw").await;

    // First login: plaintext comparison, then migration to a hash.
    let first_req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": email,
            "password": "legacy_plaintext_pw",
            "device_info": "test suite",
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, first_req).await.status(),
        StatusCode::OK
    );

    // Second login: the stored value is now a hash, so this exercises `verify_password` instead
    // of the plaintext fallback.
    let second_req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": email,
            "password": "legacy_plaintext_pw",
            "device_info": "test suite",
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, second_req).await.status(),
        StatusCode::OK
    );
}
