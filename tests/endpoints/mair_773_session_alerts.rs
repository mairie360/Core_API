//! MAIR-773: a front proxy reports a use of a session from another device. Core closes the session,
//! publishes it to every API, e-mails the agent and the administrators, and the Login screen and the
//! administration read the alert and the sign-in journal.

use crate::common::{core_db_url, fake_resend, get_raw_pool};
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::jwt_manager::{read_session_state, SessionStatus};
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};
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

macro_rules! call {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => {
                let status = resp.status();
                let body = test::read_body(resp).await;
                (
                    status,
                    serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null),
                )
            }
            Err(err) => (err.as_response_error().status_code(), Value::Null),
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

async fn create_account(raw: &PgPool) -> (u64, String) {
    let email = format!("alerts.{}@example.com", Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, status, first_connect, is_archived) \
         VALUES ('Alice', 'Martin', $1, 'active', FALSE, FALSE) RETURNING id",
    )
    .bind(&email)
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (u64::try_from(id).unwrap(), email)
}

async fn open_session(state: &web::Data<AppState>, user_id: u64) -> (Uuid, String) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&Uuid::new_v4().to_string()),
            "Firefox on Linux",
            std::net::IpAddr::from([192, 0, 2, 10]),
        ))
        .await
        .unwrap();
    (
        session_id,
        generate_session_jwt(user_id, session_id, "").unwrap(),
    )
}

fn bearer(jwt: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {jwt}"))
}

fn report(jwt: &str, level: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/sessions/report")
        .insert_header(bearer(jwt))
        .set_json(json!({ "level": level, "ip_address": "198.51.100.23", "device_info": "Thief browser" }))
}

/// A session of the seeded administrator: Core's `session_guard` refuses a JWT whose session is not in
/// the database.
async fn admin_jwt(state: &web::Data<AppState>) -> String {
    let admin = u64::try_from(*ADMIN_ID.get().unwrap()).unwrap();
    open_session(state, admin).await.1
}

#[tokio::test]
async fn mair_773_a_possible_theft_closes_the_session_and_the_next_sign_in_reads_the_alert() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (sid, jwt) = open_session(&state, user).await;

    let (status, body) = call!(app, report(&jwt, "possible").to_request());
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "revoked": true }));
    // Closed in the database and in Redis: every API refuses the JWT.
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM sessions WHERE id = $1")
            .bind(sid)
            .fetch_one(&raw)
            .await
            .unwrap();
    assert!(revoked);
    let state_in_redis = read_session_state(state.get_redis(), &sid.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state_in_redis.state, SessionStatus::Revoked);
    let (status, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/sessions/")
            .insert_header(bearer(&jwt))
            .to_request()
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The next session reads the alert, acknowledges it, and it is no longer pending.
    let (_, next) = open_session(&state, user).await;
    let pending = || {
        test::TestRequest::get()
            .uri("/api/v1/user/me/security-alert/")
            .insert_header(bearer(&next))
            .to_request()
    };
    let (status, alert) = call!(app, pending());
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alert["level"], json!("possible"));
    assert_eq!(alert["ip_address"], json!("198.51.100.23"));
    assert_eq!(alert["device_info"], json!("Thief browser"));
    let alert_id = alert["id"].as_i64().unwrap();

    let acknowledge = |id: String| {
        test::TestRequest::post()
            .uri(&format!("/api/v1/user/me/security-alert/{id}/acknowledge/"))
            .insert_header(bearer(&next))
            .to_request()
    };
    assert_eq!(
        call!(app, acknowledge("999999999".into())).0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call!(app, acknowledge("abc".into())).0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call!(app, acknowledge(alert_id.to_string())).0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call!(app, acknowledge(alert_id.to_string())).0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(call!(app, pending()).0, StatusCode::NO_CONTENT);

    // The administration lists it, read, with its agent, and the journal holds the use.
    let (status, alerts) = call!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/admin/security-alerts/")
            .insert_header(bearer(&admin_jwt(&state).await))
            .to_request()
    );
    assert_eq!(status, StatusCode::OK);
    let listed = alerts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"].as_i64() == Some(alert_id))
        .unwrap();
    assert_eq!(listed["user"]["id"].as_u64(), Some(user));
    assert!(listed["acknowledged_at"].is_string());
    let (status, logs) = call!(
        app,
        test::TestRequest::get()
            .uri(&format!("/api/v1/admin/users/{user}/connection-logs"))
            .insert_header(bearer(&admin_jwt(&state).await))
            .to_request()
    );
    assert_eq!(status, StatusCode::OK);
    assert!(logs
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line["action"] == json!("SUSPICIOUS_IP_CHANGE")));
}

#[tokio::test]
async fn mair_773_a_network_change_with_a_valid_proof_keeps_the_session() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (sid, jwt) = open_session(&state, user).await;

    let (status, body) = call!(app, report(&jwt, "ip_change").to_request());
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "revoked": false }));
    let ip: String = sqlx::query_scalar("SELECT host(ip_address) FROM sessions WHERE id = $1")
        .bind(sid)
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(ip, "198.51.100.23", "the session follows the new address");
    let (status, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/security-alert/")
            .insert_header(bearer(&jwt))
            .to_request()
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn mair_773_invalid_reports_are_refused() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (_, jwt) = open_session(&state, user).await;

    assert_eq!(
        call!(app, report(&jwt, "stolen").to_request()).0,
        StatusCode::BAD_REQUEST
    );
    let bad_address = test::TestRequest::post()
        .uri("/api/v1/sessions/report")
        .insert_header(bearer(&jwt))
        .set_json(json!({ "level": "possible", "ip_address": "not-an-address" }))
        .to_request();
    assert_eq!(call!(app, bad_address).0, StatusCode::BAD_REQUEST);
    let control = test::TestRequest::post()
        .uri("/api/v1/sessions/report")
        .insert_header(bearer(&jwt))
        .set_json(
            json!({ "level": "possible", "ip_address": "198.51.100.23", "device_info": "a\u{0}b" }),
        )
        .to_request();
    assert_eq!(call!(app, control).0, StatusCode::BAD_REQUEST);
    // A JWT without session id cannot name the session to report.
    let legacy = mairie360_api_lib::jwt_manager::generate_jwt(&user.to_string(), "").unwrap();
    assert_eq!(
        call!(app, report(&legacy, "possible").to_request()).0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn mair_773_a_report_of_a_deleted_session_is_refused_by_the_session_guard() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user, _) = create_account(&raw).await;
    let (sid, jwt) = open_session(&state, user).await;
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(sid)
        .execute(&raw)
        .await
        .unwrap();
    assert_eq!(
        call!(app, report(&jwt, "possible").to_request()).0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn mair_773_the_connection_logs_of_an_unknown_user_answer_404() {
    let (state, _raw, _redis) = setup().await;
    let app = init_app!(state);
    let (status, _) = call!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/admin/users/999999999/connection-logs")
            .insert_header(bearer(&admin_jwt(&state).await))
            .to_request()
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_773_alerts_are_e_mailed_to_the_agent_and_the_administrators_once() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let emails = fake_resend::account_emails();
    let (user, email) = create_account(&raw).await;
    let (_, jwt) = open_session(&state, user).await;
    assert_eq!(
        call!(app, report(&jwt, "certain").to_request()).0,
        StatusCode::OK
    );

    assert_eq!(
        core_api::session_alerts::deliver_pending(&state, &emails).await,
        1
    );
    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let variables = &sent[0]["template"]["variables"];
    assert_eq!(variables["CERTAINTY"], json!("certaine"));
    assert_eq!(variables["IP_ADDRESS"], json!("198.51.100.23"));
    assert_eq!(variables["DEVICE"], json!("Thief browser"));
    assert!(sent[0].to_string().contains("account-session-alert"));
    let to_admin = fake_resend::sent_to("admin@test.com");
    assert!(to_admin
        .iter()
        .any(|e| e.to_string().contains("admin-session-alert")
            && e["template"]["variables"]["AGENT_NAME"] == json!("Alice Martin")));

    // Sent once: the sweep finds nothing more.
    assert_eq!(
        core_api::session_alerts::deliver_pending(&state, &emails).await,
        0
    );
    let sent_at: bool = sqlx::query_scalar(
        "SELECT sent_at IS NOT NULL FROM session_security_alerts WHERE user_id = $1",
    )
    .bind(i32::try_from(user).unwrap())
    .fetch_one(&raw)
    .await
    .unwrap();
    assert!(sent_at);
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_773_a_refused_e_mail_is_retried_later() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let emails = fake_resend::account_emails();
    let (user, _) = create_account(&raw).await;
    let (_, jwt) = open_session(&state, user).await;
    assert_eq!(
        call!(app, report(&jwt, "possible").to_request()).0,
        StatusCode::OK
    );

    {
        let _failing = fake_resend::FailingGuard::set();
        assert_eq!(
            core_api::session_alerts::deliver_pending(&state, &emails).await,
            0
        );
    }
    let row = sqlx::query(
        "SELECT attempts, last_error, sent_at IS NULL AS unsent FROM session_security_alerts WHERE user_id = $1",
    )
    .bind(i32::try_from(user).unwrap())
    .fetch_one(&raw)
    .await
    .unwrap();
    assert_eq!(row.get::<i32, _>("attempts"), 1);
    assert!(row.get::<Option<String>, _>("last_error").is_some());
    assert!(row.get::<bool, _>("unsent"));
    // The next attempt waits a minute: move the last one back and sweep again.
    sqlx::query("UPDATE session_security_alerts SET last_attempt_at = now() - interval '2 minutes' WHERE user_id = $1")
        .bind(i32::try_from(user).unwrap())
        .execute(&raw)
        .await
        .unwrap();
    assert_eq!(
        core_api::session_alerts::deliver_pending(&state, &emails).await,
        1
    );
}
