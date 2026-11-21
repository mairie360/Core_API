//! MAIR-696: a new e-mail address is confirmed by its owner before the account uses it.

use crate::common::core_db_url;
use crate::common::fake_resend::{self, FailingGuard, TEST_PUBLIC_URL};
use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{
    jwt_manager::generate_jwt, password::hash_password, security::JwtMiddleware, state::AppState,
};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";

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

async fn setup() -> (
    web::Data<AppState>,
    PgPool,
    impl std::any::Any, // keeps the Redis container alive
) {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    (state, get_raw_pool(host.clone()).await, redis)
}

async fn create_account(raw: &PgPool, first_name: &str) -> (i32, String) {
    let email = format!("change.{}@example.com", uuid::Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, phone_number, status, \
         first_connect, is_archived) \
         VALUES ($1, 'Test', $2, $3, '0612345678', 'active', FALSE, FALSE) RETURNING id",
    )
    .bind(first_name)
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
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

fn new_address() -> String {
    format!("moved.{}@example.com", uuid::Uuid::new_v4())
}

macro_rules! ask_change {
    ($app:expr, $user:expr, $email:expr) => {{
        let req = test::TestRequest::patch()
            .uri("/api/v1/user/me/")
            .insert_header(bearer($user))
            .set_json(json!({ "email": $email, "current_password": PASSWORD }))
            .to_request();
        test::call_service(&$app, req).await.status()
    }};
}

macro_rules! confirm {
    ($app:expr, $token:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/confirm_email")
            .set_json(json!({ "token": $token }))
            .to_request();
        test::call_service(&$app, req).await.status()
    }};
}

macro_rules! me {
    ($app:expr, $user:expr) => {{
        let req = test::TestRequest::get()
            .uri("/api/v1/user/me/")
            .insert_header(bearer($user))
            .to_request();
        let body: Value = test::read_body_json(test::call_service(&$app, req).await).await;
        body
    }};
}

async fn stored_email(raw: &PgPool, user_id: i32) -> String {
    sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(raw)
        .await
        .unwrap()
}

/// The token of the link of the confirmation e-mail sent to `to`.
fn confirmation_token(to: &str) -> String {
    let sent = fake_resend::sent_to(to);
    assert_eq!(sent.len(), 1, "{sent:?}");
    sent[0]["template"]["variables"]["CONFIRM_URL"]
        .as_str()
        .and_then(|url| url.split_once("token="))
        .map(|(_, token)| token.to_string())
        .expect("CONFIRM_URL without a token")
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_a_new_address_is_pending_until_its_link_is_used() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, old_email) = create_account(&raw, "Jeanne").await;
    let new_email = new_address();

    assert_eq!(ask_change!(app, user_id, new_email), StatusCode::OK);

    // Not written: the old address is still the account's, the new one is waiting.
    assert_eq!(stored_email(&raw, user_id).await, old_email);
    let profile = me!(app, user_id);
    assert_eq!(profile["email"], old_email);
    assert_eq!(profile["pending_email"], new_email);
    // The link goes to the new address only, built from the Resend template.
    assert!(fake_resend::sent_to(&old_email).is_empty());
    let sent = fake_resend::sent_to(&new_email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0]["template"]["id"], "account-email-change-confirm");
    let variables = &sent[0]["template"]["variables"];
    assert_eq!(variables["USER_FIRST_NAME"], "Jeanne");
    assert_eq!(variables["NEW_EMAIL"], new_email);
    assert_eq!(variables["EXPIRE_MINUTES"], 60);
    let token = confirmation_token(&new_email);
    assert_eq!(
        variables["CONFIRM_URL"],
        format!("{TEST_PUBLIC_URL}/confirm-email?token={token}")
    );
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_confirming_applies_the_address_once_and_alerts_the_old_one() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, old_email) = create_account(&raw, "Jeanne").await;
    let new_email = new_address();
    assert_eq!(ask_change!(app, user_id, new_email), StatusCode::OK);
    let token = confirmation_token(&new_email);

    assert_eq!(confirm!(app, token), StatusCode::OK);

    assert_eq!(stored_email(&raw, user_id).await, new_email);
    let profile = me!(app, user_id);
    assert_eq!(profile["email"], new_email);
    assert!(profile["pending_email"].is_null(), "{profile}");
    // The former address is told, once.
    let alerts = fake_resend::sent_to(&old_email);
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert_eq!(alerts[0]["template"]["id"], "account-email-changed");
    assert_eq!(alerts[0]["template"]["variables"]["NEW_EMAIL"], new_email);
    // The link is single-use.
    assert_eq!(confirm!(app, token), StatusCode::UNAUTHORIZED);
    assert_eq!(fake_resend::sent_to(&old_email).len(), 1);
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_an_unknown_token_is_refused() {
    let (state, _raw, _redis) = setup().await;
    let app = init_app!(state);

    assert_eq!(
        confirm!(app, "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35"),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_an_address_of_another_account_is_refused_at_both_steps() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, old_email) = create_account(&raw, "Jeanne").await;
    let (_, taken) = create_account(&raw, "Paul").await;

    // At the request: 409, nothing sent, nothing pending, even with another letter case.
    assert_eq!(ask_change!(app, user_id, taken), StatusCode::CONFLICT);
    assert_eq!(
        ask_change!(app, user_id, taken.to_uppercase()),
        StatusCode::CONFLICT
    );
    assert!(fake_resend::sent_to(&taken).is_empty());
    assert!(me!(app, user_id)["pending_email"].is_null());

    // At the confirmation: the address was free when asked, taken since.
    let wanted = new_address();
    assert_eq!(ask_change!(app, user_id, wanted), StatusCode::OK);
    let token = confirmation_token(&wanted);
    create_account_with_email(&raw, &wanted).await;
    assert_eq!(confirm!(app, token), StatusCode::CONFLICT);
    assert_eq!(stored_email(&raw, user_id).await, old_email);
    assert!(fake_resend::sent_to(&old_email).is_empty());
}

async fn create_account_with_email(raw: &PgPool, email: &str) {
    sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, status, first_connect, \
         is_archived) VALUES ('Other', 'Test', $1, $2, 'active', FALSE, FALSE)",
    )
    .bind(email)
    .bind(hash_password(PASSWORD).unwrap())
    .execute(raw)
    .await
    .unwrap();
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_a_new_request_replaces_the_pending_one() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, _) = create_account(&raw, "Jeanne").await;
    let first = new_address();
    let second = new_address();
    assert_eq!(ask_change!(app, user_id, first), StatusCode::OK);
    let first_token = confirmation_token(&first);
    assert_eq!(ask_change!(app, user_id, second), StatusCode::OK);
    let second_token = confirmation_token(&second);

    assert_eq!(me!(app, user_id)["pending_email"], second);
    assert_eq!(confirm!(app, first_token), StatusCode::UNAUTHORIZED);
    assert_eq!(confirm!(app, second_token), StatusCode::OK);
    assert_eq!(stored_email(&raw, user_id).await, second);
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_sending_the_current_address_cancels_the_request() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, old_email) = create_account(&raw, "Jeanne").await;
    let wanted = new_address();
    assert_eq!(ask_change!(app, user_id, wanted), StatusCode::OK);
    let token = confirmation_token(&wanted);

    assert_eq!(ask_change!(app, user_id, old_email), StatusCode::OK);

    assert!(me!(app, user_id)["pending_email"].is_null());
    assert_eq!(confirm!(app, token), StatusCode::UNAUTHORIZED);
    assert_eq!(stored_email(&raw, user_id).await, old_email);
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_an_unsent_confirmation_changes_nothing() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, old_email) = create_account(&raw, "Jeanne").await;
    let wanted = new_address();

    let status = {
        let _failing = FailingGuard::set();
        let req = test::TestRequest::patch()
            .uri("/api/v1/user/me/")
            .insert_header(bearer(user_id))
            .set_json(json!({
                "first_name": "Jeannette",
                "email": wanted,
                "current_password": PASSWORD
            }))
            .to_request();
        test::call_service(&app, req).await.status()
    };

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(stored_email(&raw, user_id).await, old_email);
    let profile = me!(app, user_id);
    assert!(profile["pending_email"].is_null(), "{profile}");
    // Nothing was written, not even the other fields of the same request.
    assert_eq!(profile["first_name"], "Jeanne");
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_the_other_fields_are_still_written_at_once() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, _) = create_account(&raw, "Jeanne").await;
    let wanted = new_address();

    let req = test::TestRequest::patch()
        .uri("/api/v1/user/me/")
        .insert_header(bearer(user_id))
        .set_json(json!({
            "first_name": "Jeannette",
            "email": wanted,
            "current_password": PASSWORD
        }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

    let profile = me!(app, user_id);
    assert_eq!(profile["first_name"], "Jeannette");
    assert_eq!(profile["pending_email"], wanted);
}

#[tokio::test]
#[serial(process_env, fake_resend)]
async fn mair_696_a_failed_replacement_keeps_the_request_already_delivered() {
    let (state, raw, _redis) = setup().await;
    let app = init_app!(state);
    let (user_id, _) = create_account(&raw, "Jeanne").await;
    let first = new_address();
    assert_eq!(ask_change!(app, user_id, first), StatusCode::OK);
    let first_token = confirmation_token(&first);

    let status = {
        let _failing = FailingGuard::set();
        ask_change!(app, user_id, new_address())
    };

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    // The link already delivered still works and the profile still shows its address.
    assert_eq!(me!(app, user_id)["pending_email"], first);
    assert_eq!(confirm!(app, first_token), StatusCode::OK);
    assert_eq!(stored_email(&raw, user_id).await, first);
}
