//! MAIR-225: `forgot_password` must not reveal whether an e-mail matches an account.

use crate::common::fake_resend::{self, FailingGuard, TEST_PUBLIC_URL};
use crate::common::{
    core_db_url, get_pool, get_raw_pool, users::archive_user, users::create_user,
    users::unique_marker,
};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;
use serial_test::serial;

macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .app_data(fake_resend::account_emails())
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

macro_rules! forgot {
    ($app:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/forgot_password")
            .set_json(json!({ "email": $email }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await;
        (status, body)
    }};
}

#[tokio::test]
#[serial]
async fn known_and_unknown_emails_get_the_same_answer() {
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);

    let pool = get_pool().await;
    let marker = unique_marker("forgot");
    let user_id = create_user(&pool, "Known", &marker).await;
    let known_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&crate::common::get_raw_pool(host.clone()).await)
        .await
        .unwrap();
    let unknown_email = format!("nobody.{marker}@example.com");

    let unknown = forgot!(app, unknown_email);
    let known = forgot!(app, known_email);
    // A reset is now pending for the known address: a second request must look the same too.
    let pending = forgot!(app, known_email);

    assert_eq!(unknown.0, StatusCode::OK, "{:?}", unknown.1);
    assert_eq!(known, unknown, "known e-mail answered differently");
    assert_eq!(pending, unknown, "pending reset answered differently");

    // The known address really got a reset token.
    let token: Option<String> = state
        .get_redis()
        .secure_get(&format!("{known_email}/forgot_password_token"))
        .await
        .unwrap();
    let token = token.expect("no reset token for the known address");
    // ...and exactly one e-mail, the Resend template, whose link carries that token.
    let sent = fake_resend::sent_to(&known_email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(fake_resend::reset_token(&sent[0]), token);
    assert!(fake_resend::sent_to(&unknown_email).is_empty());
    let token: Option<String> = state
        .get_redis()
        .secure_get(&format!("{unknown_email}/forgot_password_token"))
        .await
        .unwrap();
    assert!(token.is_none());
}

async fn known_email(host: &str, marker: &str) -> (String, i32) {
    let pool = get_pool().await;
    let user_id = create_user(&pool, "Jeanne", marker).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&get_raw_pool(host.to_string()).await)
        .await
        .unwrap();
    (email, user_id)
}

/// MAIR-522: the e-mail is the Resend template `account-password-reset`, sent from
/// `no-reply@<EMAIL_DOMAIN>`, with a 15-minute link to the login front.
#[tokio::test]
#[serial]
async fn the_reset_email_is_the_resend_template_from_no_reply() {
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (email, _) = known_email(host, &unique_marker("template")).await;

    let (status, _) = forgot!(app, email);

    assert_eq!(status, StatusCode::OK);
    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    let body = &sent[0];
    assert_eq!(body["from"], "Mairie 360 <no-reply@mairie360.test>");
    assert!(body.get("reply_to").is_none(), "no Reply-To: {body}");
    assert!(
        body.get("text").is_none() && body.get("html").is_none(),
        "no hand-written body"
    );
    assert_eq!(body["template"]["id"], "account-password-reset");
    let variables = &body["template"]["variables"];
    assert_eq!(variables["USER_FIRST_NAME"], "Jeanne");
    assert_eq!(variables["EXPIRE_MINUTES"], 15);
    assert_eq!(variables["MAIRIE_NAME"], "Test");
    let token = fake_resend::reset_token(body);
    assert_eq!(
        variables["RESET_URL"],
        format!("{TEST_PUBLIC_URL}/reset-password?token={token}")
    );
}

#[tokio::test]
#[serial]
async fn an_archived_account_gets_no_email() {
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (email, user_id) = known_email(host, &unique_marker("archived")).await;
    archive_user(&get_raw_pool(host.clone()).await, user_id).await;

    let (status, _) = forgot!(app, email);

    assert_eq!(status, StatusCode::OK);
    assert!(fake_resend::sent_to(&email).is_empty());
}

/// When Resend refuses the e-mail, the token nobody received is dropped: the user can ask again
/// at once instead of waiting for it to expire.
#[tokio::test]
#[serial]
async fn an_unsent_email_answers_500_and_can_be_requested_again() {
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let (email, _) = known_email(host, &unique_marker("unsent")).await;

    {
        let _failing = FailingGuard::set();
        let (status, body) = forgot!(app, email);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body:?}");
    }
    let token: Option<String> = state
        .get_redis()
        .secure_get(&format!("{email}/forgot_password_token"))
        .await
        .unwrap();
    assert!(token.is_none(), "the unsent token must be dropped");

    let (status, _) = forgot!(app, email);
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fake_resend::sent_to(&email).len(), 1);
}

/// Without a mailer registered, a known address answers `500` and leaves no token behind.
#[tokio::test]
#[serial]
async fn without_a_mailer_a_known_address_answers_500() {
    let (_container, host) = get_shared_db().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = test::init_service(
        App::new().app_data(state.clone()).service(
            web::scope("/api")
                .wrap(actix_web::middleware::from_fn(session_guard))
                .wrap(JwtMiddleware)
                .configure(config),
        ),
    )
    .await;
    let (email, _) = known_email(host, &unique_marker("nomailer")).await;

    let (status, _) = forgot!(app, email);

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let token: Option<String> = state
        .get_redis()
        .secure_get(&format!("{email}/forgot_password_token"))
        .await
        .unwrap();
    assert!(token.is_none());
}
