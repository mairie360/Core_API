//! MAIR-757: the password policy (CNIL 2022) on every route that sets a password. A password of
//! fewer than 12 characters, a commonly used one, or one containing the account's e-mail, first
//! name or last name answers `400` naming the rule; the one-time tokens stay usable after a
//! refusal so the agent can try again.

use crate::common::{core_db_url, get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
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

fn create(email: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/admin/users/")
        .insert_header(admin_bearer())
        .set_json(json!({
            "first_name": "Camille",
            "last_name": "Lefebvre",
            "email": email,
            "password": password,
        }))
}

#[tokio::test]
async fn mair_757_account_creation_names_the_broken_rule() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(
        AppState::new("redis://127.0.0.1:6379".to_string(), core_db_url().await).await,
    );
    let app = init_app!(state);
    let email = format!(
        "camille.lefebvre.{}@mairie.example.com",
        Uuid::new_v4().simple()
    );

    for (password, rule) in [
        ("Velo-2026-x", "at least 12 characters"),
        ("Unbelievable", "commonly used password"),
        ("Lefebvre-Mairie-2026", "last name"),
        ("CAMILLE-velo-2026", "first name"),
    ] {
        let (status, body) = call!(app, create(&email, password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{password}: {body}");
        assert!(body.contains(rule), "{password}: {body}");
    }
    let (status, body) = call!(app, create(&email, "Ecole-Jardin-Pont-2026"));
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn mair_757_first_connection_change_refuses_the_name_and_keeps_the_token() {
    std::sync::LazyLock::force(&INIT);
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let user = create_user(&get_pool().await, "Gaspard", &unique_marker("force")).await;
    let token = Uuid::new_v4().to_string();
    state
        .get_redis()
        .set_ex(
            &format!("{token}/first_connection_id"),
            user.to_string().as_str(),
            900,
        )
        .await
        .unwrap();
    let change = |password: &str| {
        test::TestRequest::post()
            .uri("/api/v1/auth/force_change_password")
            .set_json(json!({ "token": token, "new_password": password }))
    };

    let (status, body) = call!(app, change("gaspard-de-la-mairie"));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("first name"), "{body}");
    let (status, body) = call!(app, change("Ecole-Jardin-Pont-2026"));
    assert_eq!(
        status,
        StatusCode::OK,
        "the token must survive the refusal: {body}"
    );
}

#[tokio::test]
async fn mair_757_password_reset_refuses_the_email_and_keeps_the_link() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let marker = unique_marker("reset");
    let user = create_user(&get_pool().await, "Hortense", &marker).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(&crate::common::get_raw_pool(host).await)
        .await
        .unwrap();
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    redis
        .set_ex(
            &format!("{token}/forgot_password_email"),
            email.as_str(),
            900,
        )
        .await
        .unwrap();
    redis
        .set_ex(
            &format!("{email}/forgot_password_token"),
            token.as_str(),
            900,
        )
        .await
        .unwrap();
    let reset = |password: String| {
        test::TestRequest::post()
            .uri("/api/v1/auth/reset_password")
            .set_json(json!({ "token": token, "new_password": password, "device_info": "test" }))
    };

    let local_part = email.split('@').next().unwrap().to_uppercase();
    let (status, body) = call!(app, reset(format!("{local_part}!2026")));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("e-mail"), "{body}");
    let (status, body) = call!(app, reset("Ecole-Jardin-Pont-2026".to_string()));
    assert_eq!(
        status,
        StatusCode::OK,
        "the link must survive the refusal: {body}"
    );
    let session: Value = serde_json::from_str(&body).unwrap();
    assert!(session["refresh_token"].is_string());
}

#[tokio::test]
async fn mair_757_administrator_reset_applies_the_policy() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(
        AppState::new("redis://127.0.0.1:6379".to_string(), core_db_url().await).await,
    );
    let app = init_app!(state);
    let marker = unique_marker("admin");
    let user = create_user(&get_pool().await, "Isidore", &marker).await;
    let set = |password: &str| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/admin/users/{user}/password"))
            .insert_header(admin_bearer())
            .set_json(json!({ "new_password": password }))
    };

    for (password, rule) in [
        ("short-pass", "at least 12 characters"),
        ("Isidore-Mairie-2026", "first name"),
    ] {
        let (status, body) = call!(app, set(password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{password}: {body}");
        assert!(body.contains(rule), "{password}: {body}");
    }
    let (status, body) = call!(app, set("Ecole-Jardin-Pont-2026"));
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    // The account patch checks the identity the account will have: its new last name too.
    let patch = |body: Value| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/admin/users/{user}/"))
            .insert_header(admin_bearer())
            .set_json(body)
    };
    let (status, body) = call!(
        app,
        patch(json!({ "password": format!("{marker}-Mairie-2026") }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("last name"), "{body}");
    let (status, body) = call!(
        app,
        patch(json!({ "last_name": "Rivoire", "password": "Rivoire-Mairie-2026" }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("last name"), "{body}");
    let (status, _) = call!(
        app,
        test::TestRequest::patch()
            .uri("/api/v1/admin/users/2147483000/password")
            .insert_header(admin_bearer())
            .set_json(json!({ "new_password": "Ecole-Jardin-Pont-2026" }))
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
}
