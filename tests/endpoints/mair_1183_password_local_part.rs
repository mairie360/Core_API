//! MAIR-1183: a password may contain the local part of the account's e-mail address. The
//! administrator creates an account with a temporary password holding one, the user changes it at
//! the first connection for another one holding it, then signs in with it; the other rules of the
//! policy (length, common passwords, first name, last name) still refuse a password. An
//! administrator never sets the password of an existing account: the change goes through
//! `force_change_password`.

use crate::common::{core_db_url, redis_url};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
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
    ($app:expr, $req:expr) => {{
        let resp = test::call_service(&$app, $req.to_request()).await;
        let status = resp.status();
        let body = String::from_utf8_lossy(&test::read_body(resp).await).into_owned();
        (status, body)
    }};
}

fn admin_bearer() -> (&'static str, String) {
    std::sync::LazyLock::force(&INIT);
    let admin = ADMIN_ID.get().expect("seeded admin");
    let token = generate_jwt(&admin.to_string(), "admin").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

/// A unique local part made of letters only, so it never contains the first or last name below.
fn local_part() -> String {
    let suffix: String = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .take(8)
        .collect();
    format!("agent{suffix}")
}

fn create(email: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/admin/users/")
        .insert_header(admin_bearer())
        .set_json(json!({
            "first_name": "Dominique",
            "last_name": "Fontaine",
            "email": email,
            "password": password,
        }))
}

fn change(token: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/auth/force_change_password")
        .set_json(json!({ "token": token, "new_password": password }))
}

fn login(email: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({ "email": email, "password": password, "device_info": "test suite" }))
}

/// Signs in with the temporary password: the first connection answers 412 and its one-time token.
macro_rules! first_connection_token {
    ($app:expr, $email:expr, $password:expr) => {{
        let (status, body) = call!($app, login($email, $password));
        assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
        serde_json::from_str::<Value>(&body).unwrap()["token"]
            .as_str()
            .expect("first-connection token")
            .to_string()
    }};
}

#[tokio::test]
async fn mair_1183_a_password_containing_the_email_local_part_is_accepted() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(redis_url().await, core_db_url().await).await);
    let app = init_app!(state);
    let local = local_part();
    let email = format!("{local}@mairie360.test");

    let temporary = format!("Clef_{local}_2026");
    let (status, body) = call!(app, create(&email, &temporary));
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let token = first_connection_token!(app, &email, &temporary);
    let final_password = format!("{}-Velo-2026", local.to_uppercase());
    let (status, body) = call!(app, change(&token, &final_password));
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = call!(app, login(&email, &final_password));
    assert_eq!(
        status,
        StatusCode::OK,
        "the changed password opens a session, with no first-connection step: {body}"
    );
    let session: Value = serde_json::from_str(&body).unwrap();
    assert!(session["refresh_token"].is_string(), "{body}");
}

#[tokio::test]
async fn mair_1183_the_other_rules_still_refuse_a_password_containing_the_local_part() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(redis_url().await, core_db_url().await).await);
    let app = init_app!(state);
    let local = local_part();
    let email = format!("{local}@mairie360.test");

    for (password, rule) in [
        (format!("{local}-Dominique"), "first name"),
        (format!("{local}-Fontaine"), "last name"),
        (
            local[..local.len().min(11)].to_string(),
            "at least 12 characters",
        ),
    ] {
        let (status, body) = call!(app, create(&email, &password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{rule}: {body}");
        assert!(body.contains(rule), "{rule}: {body}");
    }

    let temporary = format!("Clef_{local}_2026");
    let (status, body) = call!(app, create(&email, &temporary));
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let token = first_connection_token!(app, &email, &temporary);
    for (password, rule) in [
        (format!("{local}-dominique"), "first name"),
        (format!("FONTAINE-{local}"), "last name"),
    ] {
        let (status, body) = call!(app, change(&token, &password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{rule}: {body}");
        assert!(body.contains(rule), "{rule}: {body}");
    }
}
