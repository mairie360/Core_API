//! Account invitation: creating an account through `POST /api/v1/admin/users/` sends Core's
//! invitation e-mail (Resend template `account-invitation`), a link that sets the password through
//! `POST /api/v1/auth/reset_password`.

use crate::common::fake_resend::{self, FailingGuard, TEST_PUBLIC_URL};
use crate::common::users::unique_marker;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::session_jwt::{generate_session_jwt, user_role_name};
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Same wiring as `main.rs`, with the fake Resend.
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

async fn admin_jwt(state: &AppState) -> String {
    let admin = u64::try_from(*ADMIN_ID.get().unwrap()).unwrap();
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            admin,
            &core_api::refresh_token::hash(&format!("invitation_{session_id}")),
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    let role = user_role_name(state, admin).await.unwrap();
    generate_session_jwt(admin, session_id, &role).unwrap()
}

fn invited_email(marker: &str) -> String {
    format!("invited.{marker}@example.com")
}

macro_rules! create_account {
    ($app:expr, $jwt:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/admin/users/")
            .insert_header(("Authorization", format!("Bearer {}", $jwt)))
            .set_json(json!({
                "first_name": "Ines",
                "last_name": "Invitee",
                "email": $email,
                "password": "Provisional!123"
            }))
            .to_request();
        test::call_service(&$app, req).await
    }};
}

/// An administrator creates an account: Core e-mails the invitation, and its link sets the
/// password and opens a session; the link works once.
#[tokio::test]
#[serial(fake_resend)]
async fn the_invitation_link_sets_the_password_once() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host).await);
    let app = init_app!(state);
    let jwt = admin_jwt(&state).await;
    let email = invited_email(&unique_marker("invite"));

    let resp = create_account!(app, jwt, email);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0]["from"], "Mairie 360 <no-reply@mairie360.test>");
    assert_eq!(sent[0]["template"]["id"], "account-invitation");
    let variables = &sent[0]["template"]["variables"];
    assert_eq!(variables["USER_FIRST_NAME"], "Ines");
    assert_eq!(variables["EXPIRE_DAYS"], 7);
    assert_eq!(variables["MAIRIE_NAME"], "Test");
    let token = fake_resend::invitation_token(&sent[0]);
    assert_eq!(
        variables["SET_PASSWORD_URL"],
        format!("{TEST_PUBLIC_URL}/reset-password?token={token}")
    );

    let set_password = |token: String| {
        test::TestRequest::post()
            .uri("/api/v1/auth/reset_password")
            .set_json(json!({
                "token": token,
                "new_password": "Chosen!Pass456",
                "device_info": "test"
            }))
            .to_request()
    };
    let resp = test::call_service(&app, set_password(token.clone())).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["refresh_token"]
        .as_str()
        .is_some_and(|t| !t.is_empty()));

    let resp = test::call_service(&app, set_password(token)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "used link");

    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": email,
            "password": "Chosen!Pass456",
            "device_info": "test"
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, login).await.status(),
        StatusCode::OK
    );
}

/// A pending invitation must not silence a later "forgot password" of the same address: only the
/// `token -> e-mail` mapping is stored, not the per-address key that request checks.
#[tokio::test]
#[serial(fake_resend)]
async fn a_pending_invitation_does_not_block_forgot_password() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host).await);
    let app = init_app!(state);
    let jwt = admin_jwt(&state).await;
    let email = invited_email(&unique_marker("invite_forgot"));
    assert_eq!(
        create_account!(app, jwt, email).status(),
        StatusCode::CREATED
    );

    let pending: Option<String> = state
        .get_redis()
        .secure_get(&format!("{email}/forgot_password_token"))
        .await
        .unwrap();
    assert!(pending.is_none(), "the invitation left a per-address key");

    let forgot = test::TestRequest::post()
        .uri("/api/v1/auth/forgot_password")
        .set_json(json!({ "email": email }))
        .to_request();
    assert_eq!(
        test::call_service(&app, forgot).await.status(),
        StatusCode::OK
    );
    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 2, "invitation then reset: {sent:?}");
    assert_eq!(sent[1]["template"]["id"], "account-password-reset");
}

/// A refused e-mail never undoes the account: it is created, and no unsent token is left behind.
#[tokio::test]
#[serial(fake_resend)]
async fn a_failed_invitation_keeps_the_account() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host).await);
    let app = init_app!(state);
    let jwt = admin_jwt(&state).await;
    let email = invited_email(&unique_marker("invite_fail"));

    let _failing = FailingGuard::set();
    let resp = create_account!(app, jwt, email);

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert!(fake_resend::sent_to(&email).is_empty());
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
        .bind(&email)
        .fetch_one(&crate::common::get_raw_pool(crate::common::own_db_url().await).await)
        .await
        .unwrap();
    assert!(exists, "the account was rolled back");
}
