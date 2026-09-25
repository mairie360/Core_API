//! MAIR-264: every revocation path publishes `revoked:<sid>` to Redis with a TTL, so that the
//! other APIs refuse the JWTs of the session too.

use crate::common::{get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::database::sessions::get_active_session_ids::GetActiveSessionIdsQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::session_jwt::generate_session_jwt;
use core_api::session_revocation::{
    publish_revoked_sessions, revoked_session_key, REVOKED_SESSION_KEY_PREFIX,
};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::test_setup::redis_setup::{
    get_redis_connection, start_redis_container, RedisTestConfig,
};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use redis::Commands;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

const JWT_TIMEOUT: i64 = 3600;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Same wiring as `main.rs`.
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

macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    };
}

struct Env {
    /// Keeps the Redis container alive for the test.
    _redis: Box<dyn std::any::Any>,
    redis: RedisTestConfig,
    state: web::Data<AppState>,
    host: String,
}

async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let (node, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host.clone()).await);
    Env {
        _redis: Box::new(node),
        redis,
        state,
        host: host.clone(),
    }
}

/// Opens a session for `user_id` as the login does; returns (session id, JWT, refresh token).
async fn open_session(state: &AppState, user_id: u64) -> (Uuid, String, String) {
    let session_id = Uuid::new_v4();
    let refresh_token = format!("revocation_{}", Uuid::new_v4());
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &refresh_token,
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (
        session_id,
        generate_session_jwt(user_id, session_id).unwrap(),
        refresh_token,
    )
}

/// Asserts that `revoked:<sid>` exists with a TTL in 1..=`JWT_TIMEOUT`.
fn assert_revoked(config: &RedisTestConfig, session_id: Uuid) {
    let mut conn = get_redis_connection(config);
    let key = format!("revoked:{session_id}");
    let exists: bool = conn.exists(&key).unwrap();
    assert!(exists, "{key} missing");
    let ttl: i64 = conn.ttl(&key).unwrap();
    assert!((1..=JWT_TIMEOUT).contains(&ttl), "{key}: ttl = {ttl}");
}

fn assert_not_revoked(config: &RedisTestConfig, session_id: Uuid) {
    let mut conn = get_redis_connection(config);
    let exists: bool = conn.exists(format!("revoked:{session_id}")).unwrap();
    assert!(!exists, "session {session_id} should not be revoked");
}

async fn fresh_user(host: &str, name: &str) -> u64 {
    let pool = get_pool(host.to_string()).await;
    u64::try_from(create_user(&pool, name, &unique_marker("revocation")).await).unwrap()
}

#[core::prelude::v1::test]
fn key_is_the_one_the_lib_reads() {
    assert_eq!(REVOKED_SESSION_KEY_PREFIX, "revoked:");
    let id = Uuid::nil();
    assert_eq!(
        revoked_session_key(id),
        "revoked:00000000-0000-0000-0000-000000000000"
    );
}

#[tokio::test]
#[serial]
async fn logout_publishes_the_session() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "Logout").await;
    let (sid, jwt, _) = open_session(&env.state, user).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/sessions/logout")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::NO_CONTENT);
    assert_revoked(&env.redis, sid);
}

#[tokio::test]
#[serial]
async fn sessions_revoke_publishes_the_session() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "Revoke").await;
    let (current, jwt, _) = open_session(&env.state, user).await;
    let (other, _, other_refresh) = open_session(&env.state, user).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/sessions/revoke")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .set_json(json!({ "refresh_token": other_refresh }))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::OK);
    assert_revoked(&env.redis, other);
    assert_not_revoked(&env.redis, current);
}

#[tokio::test]
#[serial]
async fn admin_password_reset_publishes_every_session() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "AdminReset").await;
    let (first, _, _) = open_session(&env.state, user).await;
    let (second, _, _) = open_session(&env.state, user).await;
    let admin = u64::try_from(*ADMIN_ID.get().unwrap()).unwrap();
    let (_, admin_jwt, _) = open_session(&env.state, admin).await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user}/password"))
        .insert_header(("Authorization", format!("Bearer {admin_jwt}")))
        .set_json(json!({ "new_password": "a-brand-new-password" }))
        .to_request();
    let status = status!(app, req);
    assert!(status.is_success(), "{status}");
    assert_revoked(&env.redis, first);
    assert_revoked(&env.redis, second);
}

#[tokio::test]
#[serial]
async fn forgot_password_reset_publishes_the_previous_sessions() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "ForgotReset").await;
    let (previous, _, _) = open_session(&env.state, user).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(i32::try_from(user).unwrap())
        .fetch_one(&crate::common::get_raw_pool(env.host.clone()).await)
        .await
        .unwrap();
    let token = Uuid::new_v4().to_string();
    let redis = env.state.get_redis();
    redis
        .set(&format!("{token}/forgot_password_email"), email.as_str())
        .await
        .unwrap();
    redis
        .set(&format!("{email}/forgot_password_token"), token.as_str())
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset_password")
        .set_json(json!({
            "token": token,
            "new_password": "a-brand-new-password",
            "device_info": "test"
        }))
        .to_request();
    let status = status!(app, req);
    assert!(status.is_success(), "{status}");
    assert_revoked(&env.redis, previous);
}

#[tokio::test]
#[serial]
async fn force_change_password_publishes_the_previous_sessions() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "ForceChange").await;
    let (previous, _, _) = open_session(&env.state, user).await;
    let token = Uuid::new_v4().to_string();
    env.state
        .get_redis()
        .set(
            &format!("{token}/first_connection_id"),
            user.to_string().as_str(),
        )
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/force_change_password")
        .set_json(json!({ "token": token, "new_password": "a-brand-new-password" }))
        .to_request();
    let status = status!(app, req);
    assert!(status.is_success(), "{status}");
    assert_revoked(&env.redis, previous);
}

/// The admin deletion (`DELETE /api/v1/admin/users/{id}/`) reads the active sessions, deletes the
/// account, then publishes them. On this base the database has no `delete_user()` function
/// (fixed by MAIR-202), so the route cannot succeed here: the two steps are checked directly.
#[tokio::test]
#[serial]
async fn account_sessions_are_listed_and_published() {
    let env = env().await;
    let user = fresh_user(&env.host, "Archive").await;
    let (first, _, _) = open_session(&env.state, user).await;
    let (second, _, _) = open_session(&env.state, user).await;

    let mut sessions: Vec<Uuid> = env
        .state
        .get_smart_db()
        .fetch_all(&GetActiveSessionIdsQueryView::new(user))
        .await
        .unwrap();
    sessions.sort();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(sessions, expected);

    assert!(publish_revoked_sessions(env.state.get_redis(), &sessions).await);
    assert_revoked(&env.redis, first);
    assert_revoked(&env.redis, second);
}
