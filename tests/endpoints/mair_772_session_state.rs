//! MAIR-772: Core publishes the active JWT of a session (`session:<sid>`) at every refresh and writes
//! the revoked state at every revocation, so the APIs accept the current token without a database query
//! and refuse the token a refresh replaced.

use crate::common::core_db_url;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::jwt_manager::{read_session_state, session_state_key, SessionStatus};
use mairie360_api_lib::test_setup::redis_setup::{get_redis_connection, start_redis_container};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use redis::Commands;
use serde_json::json;
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
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

macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    };
}

fn get_sessions(jwt: &str) -> test::TestRequest {
    test::TestRequest::get()
        .uri("/api/v1/sessions/")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
}

/// Opens a session for user 1 as the login does: (session id, refresh token).
async fn open_session(state: &web::Data<AppState>) -> (Uuid, String) {
    let session_id = Uuid::new_v4();
    let refresh_token = format!("mair772_{}", Uuid::new_v4());
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            1,
            &core_api::refresh_token::hash(&refresh_token),
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (session_id, refresh_token)
}

#[tokio::test]
async fn mair_772_a_refresh_publishes_the_new_token_and_the_replaced_one_is_refused() {
    std::sync::LazyLock::force(&INIT);
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);

    let (sid, refresh_token) = open_session(&state).await;
    // Another role than the one the refresh reads, so the two tokens differ within the same second.
    let previous = generate_session_jwt(1, sid, "previous").unwrap();
    // Key missing: the database decides, and the session is cached without a token hash.
    assert_eq!(
        status!(app, get_sessions(&previous).to_request()),
        StatusCode::OK
    );
    let cached = read_session_state(state.get_redis(), &sid.to_string())
        .await
        .unwrap()
        .unwrap();
    assert!(cached.jwt.is_none());

    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(REFRESH_PATH)
            .set_json(json!({ "refresh_token": refresh_token }))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let refreshed = resp
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap()
        .to_string();
    assert_ne!(refreshed, previous);

    assert_eq!(
        status!(app, get_sessions(&refreshed).to_request()),
        StatusCode::OK
    );
    assert_eq!(
        status!(app, get_sessions(&previous).to_request()),
        StatusCode::UNAUTHORIZED
    );

    // The key holds the hash of the new token, never the token, for at most 15 minutes.
    let mut conn = get_redis_connection(&redis);
    let raw: String = conn.get(session_state_key(&sid.to_string())).unwrap();
    assert!(!raw.contains(&refreshed));
    let ttl: i64 = conn.ttl(session_state_key(&sid.to_string())).unwrap();
    assert!((1..=900).contains(&ttl), "ttl = {ttl}");
}

#[tokio::test]
async fn mair_772_logout_writes_the_revoked_state() {
    std::sync::LazyLock::force(&INIT);
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);

    let (sid, _) = open_session(&state).await;
    let jwt = generate_session_jwt(1, sid, "").unwrap();
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::OK
    );

    let logout = test::TestRequest::post()
        .uri("/api/v1/sessions/logout")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    assert_eq!(status!(app, logout), StatusCode::NO_CONTENT);

    let revoked = read_session_state(state.get_redis(), &sid.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(revoked.state, SessionStatus::Revoked);
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn mair_772_a_session_revoked_in_the_database_only_is_refused() {
    std::sync::LazyLock::force(&INIT);
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);

    let (sid, _) = open_session(&state).await;
    let jwt = generate_session_jwt(1, sid, "").unwrap();
    // Revoked by another writer (raw SQL): nothing in Redis, the database decides.
    let pool = crate::common::get_raw_pool(crate::common::own_db_url().await).await;
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1")
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::UNAUTHORIZED
    );
    assert!(read_session_state(state.get_redis(), &sid.to_string())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn mair_772_a_refresh_redis_cannot_publish_is_rolled_back() {
    std::sync::LazyLock::force(&INIT);
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    // Same database, Redis unreachable.
    let down =
        web::Data::new(AppState::new("redis://127.0.0.1:1".to_string(), core_db_url().await).await);
    let (_, refresh_token) = open_session(&state).await;
    let refresh = || {
        test::TestRequest::post()
            .uri(REFRESH_PATH)
            .set_json(json!({ "refresh_token": refresh_token }))
            .to_request()
    };

    let app_down = init_app!(down);
    assert_eq!(
        status!(app_down, refresh()),
        StatusCode::SERVICE_UNAVAILABLE
    );
    // Nothing was rotated: the same refresh token still works once Redis answers.
    let app = init_app!(state);
    assert_eq!(status!(app, refresh()), StatusCode::OK);
}
