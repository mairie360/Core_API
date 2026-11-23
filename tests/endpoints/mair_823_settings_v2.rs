//! MAIR-823 (Settings V2): the caller ends one of their other sessions or all of them, changes
//! their own password, and gets a sign-in alert when they asked for one.

use crate::common::core_db_url;
use crate::common::fake_resend;
use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::rate_limit::RateLimits;
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{password::hash_password, security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";
const NEW_PASSWORD: &str = "Changed!Pass456";

macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .app_data(web::Data::new(RateLimits::default()))
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

async fn setup() -> (web::Data<AppState>, PgPool, impl std::any::Any) {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    (state, get_raw_pool(host).await, redis)
}

/// A fresh active account holding `PASSWORD`, returned with its e-mail address.
async fn create_account(raw: &PgPool) -> (u64, String) {
    let email = format!("settings.{}@example.com", Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, status, first_connect, \
         is_archived) VALUES ('Alice', 'Test', $1, $2, 'active', FALSE, FALSE) RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (u64::try_from(id).unwrap(), email)
}

/// Opens a session for `user_id` as the login does; returns its id and its JWT.
async fn open_session(state: &web::Data<AppState>, user_id: u64) -> (Uuid, String) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&Uuid::new_v4().to_string()),
            "Firefox on Linux",
            std::net::IpAddr::from([203, 0, 113, 7]),
        ))
        .await
        .unwrap();
    (
        session_id,
        generate_session_jwt(user_id, session_id, "").unwrap(),
    )
}

async fn is_revoked(raw: &PgPool, session_id: Uuid) -> bool {
    sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(raw)
        .await
        .unwrap()
}

fn delete_session(jwt: &str, session_id: &str) -> test::TestRequest {
    test::TestRequest::delete()
        .uri(&format!("/api/v1/sessions/{session_id}/"))
        .insert_header(("Authorization", format!("Bearer {jwt}")))
}

fn change_password(jwt: &str, current: &str, new: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/user/me/password")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .set_json(json!({ "current_password": current, "new_password": new }))
}

#[tokio::test]
async fn mair_823_revoke_one_other_session_by_id() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (current, jwt) = open_session(&state, user).await;
    let (other, other_jwt) = open_session(&state, user).await;

    assert_eq!(
        status!(app, delete_session(&jwt, &other.to_string()).to_request()),
        StatusCode::NO_CONTENT
    );
    assert!(is_revoked(&raw, other).await);
    assert!(!is_revoked(&raw, current).await);
    // The revoked session's JWT is refused from now on.
    let req = test::TestRequest::get()
        .uri("/api/v1/sessions/")
        .insert_header(("Authorization", format!("Bearer {other_jwt}")))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::UNAUTHORIZED);
    // Already revoked: the same 404 as an unknown id.
    assert_eq!(
        status!(app, delete_session(&jwt, &other.to_string()).to_request()),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        status!(
            app,
            delete_session(&jwt, &Uuid::new_v4().to_string()).to_request()
        ),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn mair_823_revoke_by_id_refuses_the_current_and_foreign_sessions() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (stranger, _) = create_account(&raw).await;
    let (current, jwt) = open_session(&state, user).await;
    let (foreign, _) = open_session(&state, stranger).await;

    assert_eq!(
        status!(app, delete_session(&jwt, &current.to_string()).to_request()),
        StatusCode::CONFLICT
    );
    assert_eq!(
        status!(app, delete_session(&jwt, &foreign.to_string()).to_request()),
        StatusCode::NOT_FOUND
    );
    assert!(!is_revoked(&raw, current).await);
    assert!(!is_revoked(&raw, foreign).await);
    assert_eq!(
        status!(app, delete_session(&jwt, "not-a-uuid").to_request()),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn mair_823_revoke_others_keeps_the_current_session() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (current, jwt) = open_session(&state, user).await;
    let (a, _) = open_session(&state, user).await;
    let (b, _) = open_session(&state, user).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/sessions/revoke-others")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body, json!({ "revoked": 2 }));
    assert!(is_revoked(&raw, a).await);
    assert!(is_revoked(&raw, b).await);
    assert!(!is_revoked(&raw, current).await);

    // Nothing left to revoke.
    let req = test::TestRequest::post()
        .uri("/api/v1/sessions/revoke-others")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body, json!({ "revoked": 0 }));
}

#[tokio::test]
async fn mair_823_change_password_revokes_the_other_sessions() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (current, jwt) = open_session(&state, user).await;
    let (other, _) = open_session(&state, user).await;

    assert_eq!(
        status!(
            app,
            change_password(&jwt, PASSWORD, NEW_PASSWORD).to_request()
        ),
        StatusCode::NO_CONTENT
    );
    let hash: String = sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
        .bind(i32::try_from(user).unwrap())
        .fetch_one(&raw)
        .await
        .unwrap();
    assert!(mairie360_api_lib::password::verify_password(NEW_PASSWORD, &hash).unwrap());
    assert!(is_revoked(&raw, other).await);
    assert!(!is_revoked(&raw, current).await);
}

#[tokio::test]
async fn mair_823_change_password_refusals() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (_, jwt) = open_session(&state, user).await;

    // Shorter than 8 characters: refused before any check.
    assert_eq!(
        status!(app, change_password(&jwt, PASSWORD, "court").to_request()),
        StatusCode::BAD_REQUEST
    );
    // Wrong current password.
    assert_eq!(
        status!(
            app,
            change_password(&jwt, "Wrong!Pass123", NEW_PASSWORD).to_request()
        ),
        StatusCode::FORBIDDEN
    );
    // Same password as the current one.
    assert_eq!(
        status!(app, change_password(&jwt, PASSWORD, PASSWORD).to_request()),
        StatusCode::BAD_REQUEST
    );
    // Unknown field.
    let req = test::TestRequest::post()
        .uri("/api/v1/user/me/password")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .set_json(json!({ "current_password": PASSWORD, "new_password": NEW_PASSWORD, "x": 1 }))
        .to_request();
    assert_eq!(status!(app, req), StatusCode::BAD_REQUEST);
    // The budget (5 checks per 15 minutes) is spent: the next check is refused with Retry-After.
    for _ in 0..3 {
        let _ = status!(
            app,
            change_password(&jwt, "Wrong!Pass123", NEW_PASSWORD).to_request()
        );
    }
    let resp = test::try_call_service(
        &app,
        change_password(&jwt, PASSWORD, NEW_PASSWORD).to_request(),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp.headers().contains_key("Retry-After"));
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_823_login_alert_is_sent_only_when_asked() {
    let (state, raw, _redis) = setup().await;
    let emails = fake_resend::account_emails();
    let (user, email) = create_account(&raw).await;

    // Not asked: nothing is sent.
    core_api::login_alert::notify(&state, Some(&emails), user, "Firefox", "203.0.113.7").await;
    assert!(fake_resend::sent_to(&email).is_empty());

    sqlx::query("INSERT INTO user_notification_settings (user_id, login_alerts) VALUES ($1, TRUE)")
        .bind(i32::try_from(user).unwrap())
        .execute(&raw)
        .await
        .unwrap();
    core_api::login_alert::notify(&state, Some(&emails), user, "Firefox", "203.0.113.7").await;
    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let variables = &sent[0]["template"]["variables"];
    assert_eq!(variables["USER_FIRST_NAME"], json!("Alice"));
    assert_eq!(variables["DEVICE"], json!("Firefox"));
    assert_eq!(variables["IP_ADDRESS"], json!("203.0.113.7"));
    assert!(sent[0].to_string().contains("account-login-alert"));

    // A refusal of Resend is only logged.
    let _failing = fake_resend::FailingGuard::set();
    core_api::login_alert::notify(&state, Some(&emails), user, "Firefox", "203.0.113.7").await;
}

#[tokio::test]
async fn mair_823_new_preferences_through_the_routes() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (_, jwt) = open_session(&state, user).await;

    let req = test::TestRequest::patch()
        .uri("/api/v1/user/me/preferences/")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .set_json(json!({ "autosave": true }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body["autosave"], json!(true));

    let req = test::TestRequest::patch()
        .uri("/api/v1/user/me/notifications/")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .set_json(json!({ "system_updates": false, "login_alerts": true }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body["system_updates"], json!(false));
    assert_eq!(body["login_alerts"], json!(true));

    let req = test::TestRequest::get()
        .uri("/api/v1/user/me/notifications/")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body["login_alerts"], json!(true));
    assert_eq!(body["email"], Value::Null);
}
