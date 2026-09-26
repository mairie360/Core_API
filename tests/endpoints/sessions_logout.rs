//! MAIR-226: `POST /api/v1/sessions/logout` revokes the JWT's session from the JWT alone.

use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::session_jwt::{decode_session_jwt, generate_session_jwt};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::{
    security::JwtMiddleware,
    state::AppState,
    test_setup::{queries_setup::get_shared_db, redis_setup::start_redis_container},
};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

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

fn get_sessions(jwt: &str) -> test::TestRequest {
    test::TestRequest::get()
        .uri("/api/v1/sessions/")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
}

fn logout(jwt: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/sessions/logout")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
}

/// Opens a session for user 1 as the login does and returns (JWT, refresh token).
async fn open_session(state: &web::Data<AppState>) -> (String, String) {
    let session_id = Uuid::new_v4();
    let refresh_token = format!("logout_{}", Uuid::new_v4());
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            1,
            &refresh_token,
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (generate_session_jwt(1, session_id).unwrap(), refresh_token)
}

#[tokio::test]
#[serial]
async fn logout_revokes_the_jwt_and_its_refresh_token() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), host.clone()).await);
    let app = init_app!(state);

    let (jwt, refresh_token) = open_session(&state).await;
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::OK
    );

    assert_eq!(
        status!(app, logout(&jwt).to_request()),
        StatusCode::NO_CONTENT
    );

    // The same JWT is refused...
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::UNAUTHORIZED
    );
    // ...and so is its refresh token.
    let refresh = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": refresh_token }))
        .to_request();
    assert_eq!(status!(app, refresh), StatusCode::UNAUTHORIZED);

    // Logging out again does not fail.
    assert_eq!(
        status!(app, logout(&jwt).to_request()),
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
#[serial]
async fn logout_keeps_the_other_sessions() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), host.clone()).await);
    let app = init_app!(state);

    let (laptop, _) = open_session(&state).await;
    let (phone, _) = open_session(&state).await;

    assert_eq!(
        status!(app, logout(&laptop).to_request()),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        status!(app, get_sessions(&laptop).to_request()),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status!(app, get_sessions(&phone).to_request()),
        StatusCode::OK
    );
}

#[tokio::test]
#[serial]
async fn refresh_keeps_the_session_bound_jwt() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), host.clone()).await);
    let app = init_app!(state);

    let (jwt, refresh_token) = open_session(&state).await;
    let req = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": refresh_token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let refreshed = resp
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap()
        .to_string();
    assert_eq!(
        decode_session_jwt(&refreshed).unwrap().session_id(),
        decode_session_jwt(&jwt).unwrap().session_id()
    );

    // Logging out with the refreshed JWT also kills the original one (same session).
    assert_eq!(
        status!(app, logout(&refreshed).to_request()),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        status!(app, get_sessions(&jwt).to_request()),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[serial]
async fn legacy_jwt_without_session_id_still_works() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    let legacy = generate_jwt("1", "").unwrap();
    assert_eq!(
        status!(app, logout(&legacy).to_request()),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        status!(app, get_sessions(&legacy).to_request()),
        StatusCode::OK
    );
}

#[tokio::test]
#[serial]
async fn logout_requires_a_jwt() {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    let app = init_app!(state);

    let req = test::TestRequest::post()
        .uri("/api/v1/sessions/logout")
        .to_request();
    assert_eq!(status!(app, req), StatusCode::UNAUTHORIZED);
}
