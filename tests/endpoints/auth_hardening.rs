//! Authorization and authentication fixes of the 2026-10-02 audit (MAIR-390).

use crate::common::acl_redis::AclRedis;
use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::rate_limit::RateLimits;
use core_api::refresh_token;
use mairie360_api_lib::{
    jwt_manager::generate_jwt, password::hash_password, security::JwtMiddleware, state::AppState,
    test_setup::queries_setup::get_shared_db,
};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "auth_hardening_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";

/// Same wiring as `main.rs`, optionally with the rate limits.
macro_rules! init_app {
    ($state:expr) => {
        init_app!($state, App::new())
    };
    ($state:expr, $app:expr) => {
        test::init_service(
            $app.app_data($state.clone())
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

async fn setup() -> (web::Data<AppState>, PgPool) {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:6379".to_string(), host.clone()).await);
    (state, get_raw_pool(host.clone()).await)
}

/// Inserts an account with `PASSWORD` and returns `(id, email)`.
async fn create_account(raw: &PgPool, first_connect: bool, archived: bool) -> (i32, String) {
    let email = format!("hardening.{}@example.com", uuid::Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, phone_number, status, \
         first_connect, is_archived) \
         VALUES ('Audit', 'Test', $1, $2, '0612345678', 'active', $3, $4) RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .bind(first_connect)
    .bind(archived)
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (id, email)
}

fn bearer(user_id: i32) -> (&'static str, String) {
    let token = generate_jwt(&user_id.to_string(), "user").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

fn login_request(email: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({ "email": email, "password": password, "device_info": "test" }))
}

#[tokio::test]
#[serial]
async fn first_connection_token_requires_the_password() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    // The token lives in Redis: same ACL Redis as the chart, like `redis_acl.rs`.
    let redis = AclRedis::start().await;
    std::env::remove_var("REDIS_KEY_PREFIX");
    std::env::remove_var("REDIS_USERNAME");
    std::env::set_var("REDIS_URL", redis.url_as("core-api"));
    let state = web::Data::new(AppState::new(redis.url_as("core-api"), host.clone()).await);
    let raw = get_raw_pool(host.clone()).await;
    let app = init_app!(state);
    let (_, email) = create_account(&raw, true, false).await;

    let resp = test::call_service(&app, login_request(&email, "Wrong!Pass123").to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body = test::read_body(resp).await;
    assert_eq!(body, "Invalid credentials provided.", "no token leaked");

    let resp = test::call_service(&app, login_request(&email, PASSWORD).to_request()).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    std::env::remove_var("REDIS_URL");
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    assert!(body["token"].is_string());
}

#[tokio::test]
#[serial]
async fn archived_account_cannot_log_in() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (_, email) = create_account(&raw, false, true).await;

    let resp = test::call_service(&app, login_request(&email, PASSWORD).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn encoded_admin_path_is_still_guarded() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (user_id, _) = create_account(&raw, false, false).await;

    for uri in ["/api/v1/admin/users/", "/api/v1/%61dmin/users/"] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(bearer(user_id))
            .to_request();
        let status = match test::try_call_service(&app, req).await {
            Ok(resp) => resp.status(),
            Err(error) => error.as_response_error().status_code(),
        };
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
    }
}

#[tokio::test]
#[serial]
async fn group_membership_is_added_to_the_group_of_the_path() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (owner, _) = create_account(&raw, false, false).await;
    let (other_owner, _) = create_account(&raw, false, false).await;
    let new_group = |owner: i32| {
        let raw = raw.clone();
        async move {
            sqlx::query(
                "INSERT INTO groups (owner_id, name, description) VALUES ($1, $2, 'test') \
                 RETURNING id",
            )
            .bind(owner)
            .bind(format!("hardening-{}", uuid::Uuid::new_v4()))
            .fetch_one(&raw)
            .await
            .unwrap()
            .get::<i32, _>("id")
        }
    };
    let own_group = new_group(owner).await;
    let foreign_group = new_group(other_owner).await;

    // The body cannot redirect the insertion to a group the caller has no right on.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/groups/{own_group}/users/"))
        .insert_header(bearer(owner))
        .set_json(json!({ "user_id": owner, "group_id": foreign_group }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2)",
    )
    .bind(foreign_group)
    .bind(owner)
    .fetch_one(&raw)
    .await
    .unwrap();
    assert!(!member);

    // Not a member and no `read` right: the foreign group is not readable.
    for uri in [
        format!("/api/v1/groups/{foreign_group}/"),
        format!("/api/v1/groups/{foreign_group}/users/"),
    ] {
        let req = test::TestRequest::get()
            .uri(&uri)
            .insert_header(bearer(owner))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "{uri}");
    }
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/groups/{own_group}/users/"))
        .insert_header(bearer(owner))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn refresh_tokens_are_hashed_and_rotated() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (user_id, email) = create_account(&raw, false, false).await;

    let resp = test::call_service(&app, login_request(&email, PASSWORD).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let first = body["refresh_token"].as_str().unwrap().to_string();

    let stored: Vec<String> =
        sqlx::query_scalar("SELECT token_hash FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&raw)
            .await
            .unwrap();
    assert_eq!(stored, vec![refresh_token::hash(&first)]);

    let refresh = |token: &str| {
        test::TestRequest::post()
            .uri(REFRESH_PATH)
            .set_json(json!({ "refresh_token": token }))
    };
    let resp = test::call_service(&app, refresh(&first).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let second = body["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(first, second);

    // The rotated-out token is dead, the new one works.
    let resp = test::call_service(&app, refresh(&first).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = test::call_service(&app, refresh(&second).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn other_users_record_hides_phone_and_archived_accounts() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (reader, _) = create_account(&raw, false, false).await;
    let (target, _) = create_account(&raw, false, false).await;
    let (archived, _) = create_account(&raw, false, true).await;

    let get = |id: i32, as_user: i32| {
        test::TestRequest::get()
            .uri(&format!("/api/v1/user/{id}/"))
            .insert_header(bearer(as_user))
    };
    let body: Value =
        test::read_body_json(test::call_service(&app, get(target, reader).to_request()).await)
            .await;
    assert!(body["phone"].is_null());
    let body: Value =
        test::read_body_json(test::call_service(&app, get(target, target).to_request()).await)
            .await;
    assert_eq!(body["phone"], "0612345678");
    let resp = test::call_service(&app, get(archived, reader).to_request()).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn email_change_requires_the_current_password() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (user_id, _) = create_account(&raw, false, false).await;
    let new_email = format!("moved.{}@example.com", uuid::Uuid::new_v4());

    let patch = |body: Value| {
        test::TestRequest::patch()
            .uri("/api/v1/user/me/")
            .insert_header(bearer(user_id))
            .set_json(body)
    };
    let resp = test::call_service(&app, patch(json!({ "email": new_email })).to_request()).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let resp = test::call_service(
        &app,
        patch(json!({ "email": new_email, "current_password": "Wrong!Pass123" })).to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let resp = test::call_service(
        &app,
        patch(json!({ "email": new_email, "current_password": PASSWORD })).to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn login_is_rate_limited_per_email() {
    let (state, raw) = setup().await;
    let limits = web::Data::new(RateLimits::default());
    let app = init_app!(state, App::new().app_data(limits));
    let (_, email) = create_account(&raw, false, false).await;

    for _ in 0..10 {
        let resp =
            test::call_service(&app, login_request(&email, "Wrong!Pass123").to_request()).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
    let resp = test::call_service(&app, login_request(&email, PASSWORD).to_request()).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp.headers().contains_key("retry-after"));
}
