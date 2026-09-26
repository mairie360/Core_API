//! MAIR-267: under the chart's Redis ACL (`core-api` may only use `~core-api:*` and
//! `revoked:*`), Core's one-time tokens are written as `core-api:<key>` with a TTL, and the
//! forgot-password and first-connection flows work end to end.

use crate::common::acl_redis::AclRedis;
use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::redis_keys::{FIRST_CONNECTION_TTL_SECONDS, FORGOT_PASSWORD_TTL_SECONDS};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use redis::Commands;
use serde_json::{json, Value};
use serial_test::serial;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Sets `REDIS_URL` like the chart does (credentials of the `core-api` user) for the duration of
/// a test, so Core's keys get the `core-api:` prefix; restored even if the test panics.
struct RedisUrlGuard;

impl RedisUrlGuard {
    fn set(url: &str) -> Self {
        std::env::remove_var("REDIS_KEY_PREFIX");
        std::env::remove_var("REDIS_USERNAME");
        std::env::set_var("REDIS_URL", url);
        Self
    }
}

impl Drop for RedisUrlGuard {
    fn drop(&mut self) {
        std::env::remove_var("REDIS_URL");
    }
}

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

macro_rules! call {
    ($app:expr, $req:expr) => {{
        let resp = test::call_service(&$app, $req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        (status, String::from_utf8_lossy(&body).to_string())
    }};
}

/// TTL of `key` in `1..=max`.
fn assert_ttl(conn: &mut redis::Connection, key: &str, max: u64) {
    let ttl: i64 = conn.ttl(key).unwrap();
    assert!(
        ttl > 0 && ttl.unsigned_abs() <= max,
        "{key}: ttl = {ttl} (max {max})"
    );
}

async fn email_of(host: &str, user_id: i32) -> String {
    sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&get_raw_pool(host.to_string()).await)
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
async fn forgot_password_flow_works_under_the_acl() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let redis = AclRedis::start().await;
    let _env = RedisUrlGuard::set(&redis.url_as("core-api"));
    let smtp_port = super::forgot_password::start_fake_smtp().await;
    std::env::set_var("SMTP_HOST", "localhost");
    std::env::set_var("SMTP_PORT", smtp_port.to_string());
    std::env::remove_var("SMTP_USERNAME");

    let state = web::Data::new(AppState::new(redis.url_as("core-api"), host.clone()).await);
    let app = init_app!(state);
    let pool = get_pool(host.clone()).await;
    let user = create_user(&pool, "Forgot", &unique_marker("acl")).await;
    let email = email_of(host, user).await;

    let (status, body) = call!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/auth/forgot_password")
            .set_json(json!({ "email": email }))
            .to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");

    let mut admin = redis.admin();
    let token_key = format!("core-api:{email}/forgot_password_token");
    let token: String = admin.get(&token_key).expect("prefixed token key");
    assert_ttl(&mut admin, &token_key, FORGOT_PASSWORD_TTL_SECONDS);
    let email_key = format!("core-api:{token}/forgot_password_email");
    let stored: String = admin.get(&email_key).unwrap();
    assert_eq!(stored, email);
    assert_ttl(&mut admin, &email_key, FORGOT_PASSWORD_TTL_SECONDS);
    let unprefixed: bool = admin
        .exists(format!("{email}/forgot_password_token"))
        .unwrap();
    assert!(!unprefixed);

    let (status, body) = call!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/auth/reset_password")
            .set_json(json!({
                "token": token,
                "new_password": "a-brand-new-password",
                "device_info": "test"
            }))
            .to_request()
    );
    assert!(status.is_success(), "{status}: {body}");
    let left: bool = admin.exists(&token_key).unwrap();
    assert!(!left, "the reset consumes the token");
    let left: bool = admin.exists(&email_key).unwrap();
    assert!(!left, "the reset consumes the token");
}

#[tokio::test]
#[serial]
async fn first_connection_flow_works_under_the_acl() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let redis = AclRedis::start().await;
    let _env = RedisUrlGuard::set(&redis.url_as("core-api"));

    let state = web::Data::new(AppState::new(redis.url_as("core-api"), host.clone()).await);
    let app = init_app!(state);
    let pool = get_pool(host.clone()).await;
    let user = create_user(&pool, "FirstLogin", &unique_marker("acl")).await;
    let email = email_of(host, user).await;

    let (status, body) = call!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(json!({ "email": email, "password": "password", "device_info": "test" }))
            .to_request()
    );
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    let token = serde_json::from_str::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let mut admin = redis.admin();
    let token_key = format!("core-api:{user}/first_connection_token");
    let id_key = format!("core-api:{token}/first_connection_id");
    let stored: String = admin.get(&token_key).expect("prefixed token key");
    assert_eq!(stored, token);
    assert_ttl(&mut admin, &token_key, FIRST_CONNECTION_TTL_SECONDS);
    let stored: String = admin.get(&id_key).unwrap();
    assert_eq!(stored, user.to_string());
    assert_ttl(&mut admin, &id_key, FIRST_CONNECTION_TTL_SECONDS);

    let (status, body) = call!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/auth/force_change_password")
            .set_json(json!({ "token": token, "new_password": "a-brand-new-password" }))
            .to_request()
    );
    assert!(status.is_success(), "{status}: {body}");
    let left: bool = admin.exists(&token_key).unwrap();
    assert!(!left, "the change consumes the token");
    let left: bool = admin.exists(&id_key).unwrap();
    assert!(!left, "the change consumes the token");

    // The new password now logs in.
    let (status, body) = call!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(json!({
                "email": email,
                "password": "a-brand-new-password",
                "device_info": "test"
            }))
            .to_request()
    );
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
#[serial]
async fn revocation_list_stays_unprefixed_under_the_acl() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let redis = AclRedis::start().await;
    let _env = RedisUrlGuard::set(&redis.url_as("core-api"));

    let state = web::Data::new(AppState::new(redis.url_as("core-api"), host.clone()).await);
    let sid = uuid::Uuid::new_v4();
    core_api::session_revocation::publish_revoked_session(state.get_redis(), sid, 60)
        .await
        .unwrap();

    let mut admin = redis.admin();
    assert_ttl(&mut admin, &format!("revoked:{sid}"), 60);
}
