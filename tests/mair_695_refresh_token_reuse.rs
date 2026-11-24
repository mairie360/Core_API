//! A refresh token already replaced by a rotation was only refused (`401`), so a stolen
//! token replayed after the rotation went unnoticed and the session stayed open (MAIR-695). Every
//! token a rotation replaced is now recognized (MAIR-776): within 10 seconds (concurrent renewals of
//! several fronts) it answers `409 refresh_superseded`, later the whole session is revoked, the reuse
//! is logged in `connection_logs` and the alert e-mails are queued.
//!
//! Own test binary: one test captures the logs with a thread-local `tracing` subscriber, which
//! misses events when other tests of the same process install their own subscribers in parallel.

#[allow(dead_code, unused_imports)]
mod common;

use crate::common::{core_db_url, get_raw_pool, own_db_url};
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::v1::sessions::REFRESH_PATH;
use core_api::endpoints::{config, public_config, session_guard::session_guard};
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

/// Same values as the other endpoint tests.
fn init_env() {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
}

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

/// Refreshes with `token`: returns the status and, on success, the new refresh token.
macro_rules! refresh {
    ($app:expr, $token:expr) => {
        refresh!($app, $token, json!(null))
    };
    ($app:expr, $token:expr, $device:expr) => {{
        let mut body = json!({ "refresh_token": $token });
        if !$device.is_null() {
            body["device_info"] = $device;
        }
        let request = test::TestRequest::post()
            .uri(REFRESH_PATH)
            .insert_header(("User-Agent", "mair776 user agent"))
            .set_json(body)
            .to_request();
        let response = test::call_service(&$app, request).await;
        let status = response.status();
        let token = if status == StatusCode::OK {
            let body: Value = test::read_body_json(response).await;
            Some(body["refresh_token"].as_str().unwrap().to_string())
        } else {
            None
        };
        (status, token)
    }};
}

/// Opens a session for user 1 as the login does and returns (session id, refresh token).
async fn open_session(state: &web::Data<AppState>) -> (Uuid, String) {
    let session_id = Uuid::new_v4();
    let refresh_token = format!("mair695_{}", Uuid::new_v4());
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            1,
            &core_api::refresh_token::hash(&refresh_token),
            "mair695 device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (session_id, refresh_token)
}

/// `(revoked, previous hash is set, rotated_at is set)` of the session.
async fn session_state(session_id: Uuid) -> (bool, bool, bool) {
    let raw = get_raw_pool(core_db_url().await).await;
    let row = sqlx::query(
        "SELECT revoked_at IS NOT NULL AS revoked, previous_token_hash IS NOT NULL AS has_previous, \
         rotated_at IS NOT NULL AS has_rotated FROM sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_one(&raw)
    .await
    .unwrap();
    (
        row.get("revoked"),
        row.get("has_previous"),
        row.get("has_rotated"),
    )
}

/// Moves every rotation of the session `seconds` into the past.
async fn age_rotation(session_id: Uuid, seconds: i32) {
    // Superuser: no API role reaches the replaced hashes.
    let raw = get_raw_pool(own_db_url().await).await;
    sqlx::query(
        "UPDATE session_replaced_refresh_tokens \
         SET replaced_at = replaced_at - make_interval(secs => $2) WHERE session_id = $1",
    )
    .bind(session_id)
    .bind(f64::from(seconds))
    .execute(&raw)
    .await
    .unwrap();
}

/// `(address, device)` of the `REFRESH_REUSE` lines of the session.
async fn reuse_logs(session_id: Uuid) -> Vec<(Option<String>, Option<String>)> {
    let raw = get_raw_pool(core_db_url().await).await;
    sqlx::query_as(
        "SELECT host(ip_address), device_info FROM connection_logs \
         WHERE session_id = $1 AND action_type = 'REFRESH_REUSE' ORDER BY timestamp",
    )
    .bind(session_id)
    .fetch_all(&raw)
    .await
    .unwrap()
}

/// Levels of the alerts queued for the session.
async fn alert_levels(session_id: Uuid) -> Vec<String> {
    let raw = get_raw_pool(core_db_url().await).await;
    sqlx::query_scalar("SELECT level FROM session_security_alerts WHERE session_id = $1")
        .bind(session_id)
        .fetch_all(&raw)
        .await
        .unwrap()
}

async fn on_revocation_list(redis_url: &str, session_id: Uuid) -> bool {
    let mut conn = redis::Client::open(redis_url)
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    redis::cmd("EXISTS")
        .arg(format!("revoked:{session_id}"))
        .query_async(&mut conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn mair_695_rotation_keeps_the_replaced_token_and_the_chain_goes_on() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    assert_eq!(session_state(session_id).await, (false, false, false));

    let (status, second) = refresh!(app, &first);
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session_state(session_id).await, (false, true, true));
    let (status, third) = refresh!(app, second.unwrap());
    assert_eq!(
        status,
        StatusCode::OK,
        "the new token keeps renewing the session"
    );
    assert!(third.is_some());
    assert_eq!(session_state(session_id).await, (false, true, true));
}

#[tokio::test]
async fn mair_776_previous_token_within_the_grace_window_is_superseded_without_revoking() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    let (_, second) = refresh!(app, &first);

    // A second front renewing at the same time presents the token just replaced: it is told to
    // read the cookie again instead of being signed out.
    let request = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": &first }))
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(test::read_body(response).await, "refresh_superseded");
    assert!(!session_state(session_id).await.0, "the session stays open");
    assert!(!on_revocation_list(&redis.url, session_id).await);
    assert!(reuse_logs(session_id).await.is_empty(), "nothing is logged");
    assert!(alert_levels(session_id).await.is_empty(), "no alert");
    let (status, _) = refresh!(app, second.unwrap());
    assert_eq!(status, StatusCode::OK, "the current token still works");
}

#[tokio::test]
async fn mair_695_previous_token_after_the_grace_window_revokes_the_whole_session() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    let (_, second) = refresh!(app, &first);
    age_rotation(session_id, 11).await;

    // The stolen token is replayed 11 seconds after the rotation: the bug left the session open.
    let (status, token) = refresh!(app, &first);
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "same answer as an unknown token"
    );
    assert!(token.is_none());
    assert!(session_state(session_id).await.0, "the session is revoked");
    assert!(
        on_revocation_list(&redis.url, session_id).await,
        "the other APIs refuse its JWTs"
    );
    let (status, _) = refresh!(app, second.unwrap());
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the current token is revoked too"
    );
}

#[tokio::test]
async fn mair_776_token_replaced_several_rotations_ago_revokes_the_session() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    let (_, second) = refresh!(app, &first);
    let (_, third) = refresh!(app, second.unwrap());
    age_rotation(session_id, 11).await;

    // The bug: only the token of the last rotation was kept, an older one was merely refused.
    let (status, _) = refresh!(app, &first, json!("Thief browser"));
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "same answer as an unknown token"
    );
    assert!(session_state(session_id).await.0, "the session is revoked");
    assert!(on_revocation_list(&redis.url, session_id).await);
    let (status, _) = refresh!(app, third.unwrap());
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the current token is revoked too"
    );
}

#[tokio::test]
async fn mair_776_reuse_is_logged_with_the_device_and_raises_one_certain_alert() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    let (_, _second) = refresh!(app, &first);
    age_rotation(session_id, 11).await;

    let (status, _) = refresh!(app, &first, json!("Thief browser"));
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // Replayed again: logged once more, no second alert.
    let (status, _) = refresh!(app, &first);
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let logs = reuse_logs(session_id).await;
    assert_eq!(logs.len(), 2, "{logs:?}");
    assert_eq!(
        logs[0].1.as_deref(),
        Some("Thief browser"),
        "the device sent by the BFF"
    );
    assert_eq!(
        logs[1].1.as_deref(),
        Some("mair776 user agent"),
        "the request's User-Agent when the body has none"
    );
    assert_eq!(alert_levels(session_id).await, vec!["certain".to_string()]);
}

#[tokio::test]
async fn mair_776_unknown_token_writes_nothing() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, _first) = open_session(&state).await;

    let (status, _) = refresh!(app, "mair776-never-issued");
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!session_state(session_id).await.0);
    assert!(reuse_logs(session_id).await.is_empty());
}

#[tokio::test]
async fn mair_776_device_info_is_validated() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (_session_id, first) = open_session(&state).await;

    let (status, _) = refresh!(app, &first, json!("a".repeat(513)));
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, token) = refresh!(app, &first, json!("Firefox 140"));
    assert_eq!(
        status,
        StatusCode::OK,
        "the token is still valid after the 400"
    );
    assert!(token.is_some());
}

/// Log lines written by the endpoint, captured through a test subscriber.
#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Captured {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

#[tokio::test]
async fn mair_695_reuse_is_audited_with_its_session_but_never_with_the_token() {
    init_env();
    let captured = Captured::default();
    let writer = captured.clone();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish(),
    );
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    let (_, second) = refresh!(app, &first);

    // Within the grace window: superseded, logged without the audit event.
    let (status, _) = refresh!(app, &first);
    assert_eq!(status, StatusCode::CONFLICT);
    let logs = captured.text();
    assert!(logs.contains("concurrent renewal"), "{logs}");
    assert!(!logs.contains("refresh_token_reuse"), "{logs}");

    // After it: the audit event names the session and its user.
    age_rotation(session_id, 11).await;
    let (status, _) = refresh!(app, &first);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let logs = captured.text();
    assert!(logs.contains("refresh_token_reuse"), "{logs}");
    assert!(logs.contains(&format!("session_id={session_id}")), "{logs}");
    assert!(logs.contains("user_id=1"), "{logs}");
    assert!(logs.contains("session_was_active=true"), "{logs}");
    // Neither the replayed token, its hash nor the current token is ever written.
    assert!(!logs.contains(&first), "{logs}");
    assert!(
        !logs.contains(&core_api::refresh_token::hash(&first)),
        "{logs}"
    );
    assert!(!logs.contains(&second.unwrap()), "{logs}");
}

#[tokio::test]
async fn mair_695_redis_down_retries_the_revocation_then_logs_it_as_critical() {
    init_env();
    let captured = Captured::default();
    let writer = captured.clone();
    let _guard = tracing::subscriber::set_default(
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish(),
    );
    let (redis_node, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, first) = open_session(&state).await;
    let (_, _second) = refresh!(app, &first);
    age_rotation(session_id, 11).await;

    // The revocation list is down when the stolen token comes back.
    redis_node.stop().await.unwrap();
    let started = std::time::Instant::now();
    let (status, _) = refresh!(app, &first);

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the answer does not change"
    );
    assert!(
        session_state(session_id).await.0,
        "Postgres still revokes the session"
    );
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(1_400),
        "the write is retried before giving up"
    );
    let logs = captured.text();
    assert!(logs.contains("refresh_token_reuse_unpublished"), "{logs}");
    assert!(logs.contains(&format!("session_id={session_id}")), "{logs}");
    assert!(!logs.contains(&first), "{logs}");
}
