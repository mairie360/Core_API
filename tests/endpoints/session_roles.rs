//! The JWTs issued at login and at refresh carry the caller's roles (MAIR-529): the BFFs read
//! them from the token instead of calling `BFF_user /me`.

use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::session_jwt::decode_session_jwt;
use mairie360_api_lib::{
    password::hash_password, security::JwtMiddleware, state::AppState,
    test_setup::queries_setup::get_shared_db,
};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};

const PASSWORD: &str = "Correct!Pass123";

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

async fn setup() -> (web::Data<AppState>, PgPool) {
    // Set on every test, not once: the other modules of this binary set their own secret.
    std::env::set_var("JWT_SECRET", "session_roles_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
    let (_container, host) = get_shared_db().await;
    let state = web::Data::new(AppState::new(String::new(), host.clone()).await);
    (state, get_raw_pool(host.clone()).await)
}

/// Inserts an active account with `PASSWORD` and returns `(id, email)`.
async fn create_account(raw: &PgPool) -> (i32, String) {
    let email = format!("roles.jwt.{}@example.com", uuid::Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, phone_number, status, \
         first_connect) VALUES ('Role', 'Claim', $1, $2, '0612345678', 'active', false) \
         RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (id, email)
}

/// Names of the roles `user_id` holds, ordered by role id (new accounts get `Guest` from the schema).
async fn role_names(raw: &PgPool, user_id: i32) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
         WHERE ur.user_id = $1 ORDER BY r.id",
    )
    .bind(user_id)
    .fetch_all(raw)
    .await
    .unwrap()
}

/// Creates a role named `<prefix> <uuid>`, grants it to `user_id` and returns its name.
async fn grant_new_role(raw: &PgPool, user_id: i32, prefix: &str) -> String {
    let name = format!("{prefix} {}", uuid::Uuid::new_v4().simple());
    let role_id: i32 = sqlx::query(
        "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, 'JWT test', true) \
         RETURNING id",
    )
    .bind(&name)
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(role_id)
        .execute(raw)
        .await
        .unwrap();
    name
}

fn bearer_token(resp: &actix_web::dev::ServiceResponse) -> String {
    resp.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .expect("the JWT is returned in the Authorization header")
        .to_string()
}

/// Logs in as `$email` and evaluates to `(claims of the JWT, the JWT, refresh token)`.
macro_rules! login {
    ($app:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(json!({ "email": $email, "password": PASSWORD, "device_info": "test" }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let jwt = bearer_token(&resp);
        let body: Value = test::read_body_json(resp).await;
        let refresh_token = body["refresh_token"].as_str().unwrap().to_string();
        (decode_session_jwt(&jwt).unwrap(), jwt, refresh_token)
    }};
}

#[tokio::test]
#[serial]
async fn mair_529_login_jwt_carries_every_role_of_the_user() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (user_id, email) = create_account(&raw).await;
    let agent = grant_new_role(&raw, user_id, "Agent").await;
    let manager = grant_new_role(&raw, user_id, "Manager").await;
    let expected = role_names(&raw, user_id).await;
    assert!(expected.len() >= 2 && expected.ends_with(&[agent, manager]));

    let (claims, jwt, _) = login!(app, &email);

    // Ordered by role id, `role` is the first one (like the deprecated `role` of `/user/me/`).
    assert_eq!(claims.roles(), expected);
    assert_eq!(claims.role(), expected[0]);
    // The lib, which every API uses to verify JWTs, still accepts the token.
    let lib_claims = mairie360_api_lib::jwt_manager::decode_jwt(&jwt).unwrap();
    assert_eq!(lib_claims.role(), expected[0]);
}

#[tokio::test]
#[serial]
async fn mair_529_user_without_role_gets_empty_role_claims() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (user_id, email) = create_account(&raw).await;
    sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(user_id)
        .execute(&raw)
        .await
        .unwrap();

    let (claims, jwt, _) = login!(app, &email);

    assert!(claims.roles().is_empty());
    assert_eq!(claims.role(), "");
    // `roles` is always present, an empty array rather than a missing claim.
    let payload = jwt.split('.').nth(1).unwrap();
    let payload: Value = serde_json::from_slice(
        &base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, payload)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["roles"], json!([]));
}

#[tokio::test]
#[serial]
async fn mair_529_refresh_jwt_reflects_role_changes() {
    let (state, raw) = setup().await;
    let app = init_app!(state);
    let (user_id, email) = create_account(&raw).await;
    let before = role_names(&raw, user_id).await;
    let (claims, _, refresh_token) = login!(app, &email);
    assert_eq!(claims.roles(), before);

    // Granted after the login: only the next JWT carries it.
    let granted = grant_new_role(&raw, user_id, "Admin").await;
    let req = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": refresh_token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let claims = decode_session_jwt(&bearer_token(&resp)).unwrap();

    let after = role_names(&raw, user_id).await;
    assert!(after.contains(&granted) && !before.contains(&granted));
    assert_eq!(claims.roles(), after);
    assert_eq!(claims.role(), after[0]);
}

#[tokio::test]
#[serial]
async fn mair_529_jwt_issued_before_the_roles_claim_still_decodes() {
    std::env::set_var("JWT_SECRET", "session_roles_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
    // A JWT signed by the lib (no `roles`, no `sid`), like those issued before MAIR-529.
    let jwt = mairie360_api_lib::jwt_manager::generate_jwt("1", "").unwrap();

    let claims = decode_session_jwt(&jwt).unwrap();

    assert!(claims.roles().is_empty());
    assert_eq!(claims.role(), "");
    assert_eq!(claims.user_id(), Some(1));
}
