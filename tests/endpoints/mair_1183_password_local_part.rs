//! MAIR-1183: a password may contain the local part of the account's e-mail address. The
//! administrator creates an account and sets its password with one, the user then signs in with
//! it; the other rules of the policy (length, common passwords, first name, last name) still
//! refuse a password.

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

fn reset(user: u64, password: &str) -> test::TestRequest {
    test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user}/password"))
        .insert_header(admin_bearer())
        .set_json(json!({ "new_password": password }))
}

fn login(email: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({ "email": email, "password": password, "device_info": "test suite" }))
}

#[tokio::test]
async fn mair_1183_a_password_containing_the_email_local_part_is_accepted() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(AppState::new(redis_url().await, core_db_url().await).await);
    let app = init_app!(state);
    let local = local_part();
    let email = format!("{local}@mairie360.test");

    let (status, body) = call!(app, create(&email, &format!("Clef_{local}_2026")));
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let user = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_u64()
        .unwrap();

    let final_password = format!("{}-Velo-2026", local.to_uppercase());
    let (status, body) = call!(app, reset(user, &final_password));
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");

    let (status, body) = call!(app, login(&email, &final_password));
    assert_eq!(
        status,
        StatusCode::OK,
        "the reset password opens a session, with no first-connection step: {body}"
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

    let (status, body) = call!(app, create(&email, &format!("Clef_{local}_2026")));
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let user = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_u64()
        .unwrap();
    for (password, rule) in [
        (format!("{local}-dominique"), "first name"),
        (format!("FONTAINE-{local}"), "last name"),
    ] {
        let (status, body) = call!(app, reset(user, &password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{rule}: {body}");
        assert!(body.contains(rule), "{rule}: {body}");
    }
}
