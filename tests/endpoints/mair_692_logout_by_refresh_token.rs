//! MAIR-692: logging out with an expired JWT left the refresh token valid for 30 days, because
//! both logout routes sit behind `JwtMiddleware`. `POST /api/v1/auth/logout` ends the session
//! from its refresh token alone.

use crate::common::core_db_url;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const LOGOUT_PATH: &str = "/api/v1/auth/logout";
/// Same values as the other endpoint tests: they share the test binary.
const SECRET: &str = "refresh_test_secret";

fn init_env() {
    std::env::set_var("JWT_SECRET", SECRET);
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

macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    };
}

/// Opens a session for user 1 as the login does and returns (session id, refresh token).
async fn open_session(state: &web::Data<AppState>) -> (Uuid, String) {
    let session_id = Uuid::new_v4();
    let refresh_token = format!("mair692_{}", Uuid::new_v4());
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            1,
            &core_api::refresh_token::hash(&refresh_token),
            "shared workstation",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (session_id, refresh_token)
}

/// JWT of `session_id` that expired two minutes ago.
fn expired_jwt(session_id: Uuid) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &json!({ "sub": "1", "role": "", "exp": now - 120, "sid": session_id }),
        &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap()
}

fn post(path: &str, body: &Value) -> test::TestRequest {
    test::TestRequest::post().uri(path).set_json(body)
}

fn with_jwt(request: test::TestRequest, jwt: &str) -> test::TestRequest {
    request.insert_header(("Authorization", format!("Bearer {jwt}")))
}

fn get_sessions(jwt: &str) -> test::TestRequest {
    with_jwt(test::TestRequest::get().uri("/api/v1/sessions/"), jwt)
}

#[tokio::test]
#[serial]
async fn mair_692_logout_with_an_expired_jwt_revokes_the_refresh_token() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (session_id, refresh_token) = open_session(&state).await;
    let expired = expired_jwt(session_id);
    let body = json!({ "refresh_token": refresh_token });

    // The routes behind JwtMiddleware refuse the expired JWT: the bug.
    assert_eq!(
        status!(
            app,
            with_jwt(post("/api/v1/sessions/revoke", &body), &expired).to_request()
        ),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status!(
            app,
            with_jwt(post("/api/v1/sessions/logout", &json!({})), &expired).to_request()
        ),
        StatusCode::UNAUTHORIZED
    );

    // The refresh token alone ends the session, whatever JWT comes along.
    assert_eq!(
        status!(
            app,
            with_jwt(post(LOGOUT_PATH, &body), &expired).to_request()
        ),
        StatusCode::NO_CONTENT
    );

    // The refresh token is refused afterwards...
    assert_eq!(
        status!(app, post(REFRESH_PATH, &body).to_request()),
        StatusCode::UNAUTHORIZED
    );
    // ...and so is any JWT of the session, even one that has not expired.
    let fresh = generate_session_jwt(1, session_id, "").unwrap();
    assert_eq!(
        status!(app, get_sessions(&fresh).to_request()),
        StatusCode::UNAUTHORIZED
    );
    // The other APIs learn it through the revocation list (MAIR-264).
    let mut redis_conn = redis::Client::open(redis.url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let published: bool = redis::cmd("EXISTS")
        .arg(format!("revoked:{session_id}"))
        .query_async(&mut redis_conn)
        .await
        .unwrap();
    assert!(published, "the session is on the revocation list");
}

#[tokio::test]
#[serial]
async fn mair_692_logout_answers_204_without_telling_whether_the_token_exists() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (_, refresh_token) = open_session(&state).await;
    let body = json!({ "refresh_token": refresh_token });

    // Unknown token, the real one, then the same one again: the same empty 204 each time.
    for request_body in [
        json!({ "refresh_token": format!("unknown_{}", Uuid::new_v4()) }),
        body.clone(),
        body,
    ] {
        let resp = test::call_service(&app, post(LOGOUT_PATH, &request_body).to_request()).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert!(test::read_body(resp).await.is_empty());
    }
}

#[tokio::test]
#[serial]
async fn mair_692_logout_keeps_the_other_sessions() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (_, ended) = open_session(&state).await;
    let (other_id, other) = open_session(&state).await;

    assert_eq!(
        status!(
            app,
            post(LOGOUT_PATH, &json!({ "refresh_token": ended })).to_request()
        ),
        StatusCode::NO_CONTENT
    );

    let jwt = generate_session_jwt(1, other_id, "").unwrap();
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::OK
    );
    let refresh = test::call_service(
        &app,
        post(REFRESH_PATH, &json!({ "refresh_token": other })).to_request(),
    )
    .await;
    assert_eq!(refresh.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn mair_692_logout_validates_the_body() {
    init_env();
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app = init_app!(state);

    for body in [
        json!({}),
        json!({ "refresh_token": "a".repeat(513) }),
        json!({ "refresh_token": "tab\there" }),
    ] {
        assert_eq!(
            status!(app, post(LOGOUT_PATH, &body).to_request()),
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
}

#[tokio::test]
#[serial]
async fn mair_692_logout_retried_after_a_redis_failure_publishes_the_revocation() {
    init_env();
    // Review finding: once Postgres holds the revocation, a failed Redis write must not be lost.
    let down =
        web::Data::new(AppState::new("redis://127.0.0.1:1".to_string(), core_db_url().await).await);
    let app_down = init_app!(down);
    let (session_id, refresh_token) = open_session(&down).await;
    let body = json!({ "refresh_token": refresh_token });

    // Revoked in Postgres, not published: 503, so the client retries.
    assert_eq!(
        status!(app_down, post(LOGOUT_PATH, &body).to_request()),
        StatusCode::SERVICE_UNAVAILABLE
    );

    // The retry, with Redis back, publishes the already revoked session.
    let (_redis, redis) = start_redis_container().await;
    let up = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let app_up = init_app!(up);
    assert_eq!(
        status!(app_up, post(LOGOUT_PATH, &body).to_request()),
        StatusCode::NO_CONTENT
    );
    let mut redis_conn = redis::Client::open(redis.url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let published: bool = redis::cmd("EXISTS")
        .arg(format!("revoked:{session_id}"))
        .query_async(&mut redis_conn)
        .await
        .unwrap();
    assert!(published, "the retry published the revocation");
}

#[tokio::test]
#[serial]
async fn mair_692_overlapping_logouts_both_get_the_session_to_publish() {
    init_env();
    // Review finding: a logout that waited on another one's row lock skipped the row and read a
    // snapshot taken before that revocation, so it answered 204 without publishing anything.
    let state =
        web::Data::new(AppState::new("redis://127.0.0.1:1".to_string(), core_db_url().await).await);
    let (session_id, refresh_token) = open_session(&state).await;
    let (_container, host) = mairie360_api_lib::test_setup::queries_setup::get_shared_db().await;
    let raw = crate::common::get_raw_pool(host.clone()).await;

    // A first logout revokes the session and has not committed yet.
    let mut first = raw.begin().await.unwrap();
    let first_revoked_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "UPDATE sessions SET revoked_at = now() - interval '1 minute' WHERE id = $1 RETURNING revoked_at",
    )
    .bind(session_id)
    .fetch_one(&mut *first)
    .await
    .unwrap();

    // The second one starts meanwhile and waits for the row lock.
    let view = core_api::database::sessions::revoke_session_by_token_hash::RevokeSessionByTokenHashQueryView::new(
        &core_api::refresh_token::hash(&refresh_token),
    );
    let db = state.clone();
    let second = tokio::spawn(async move { db.get_smart_db().fetch_all::<Uuid, _>(&view).await });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(
        !second.is_finished(),
        "the second logout waits for the first one"
    );
    first.commit().await.unwrap();

    assert_eq!(second.await.unwrap().unwrap(), vec![session_id]);
    // The first revocation time is kept.
    let revoked_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT revoked_at FROM sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&raw)
            .await
            .unwrap();
    assert_eq!(revoked_at, first_revoked_at);
}
