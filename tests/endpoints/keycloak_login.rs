use crate::common::keycloak_mock::{id_token_claims, sign, KeycloakMock, TestKey, REDIRECT_URI};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::keycloak::{KeycloakClient, KeycloakConfig};
use mairie360_api_lib::jwt_manager::get_user_id_from_jwt;
use mairie360_api_lib::{
    security::JwtMiddleware,
    state::AppState,
    test_setup::queries_setup::{get_shared_db, ALICE_ID},
};
use serde_json::{json, Value};
use serial_test::serial;

const KEYCLOAK_PATH: &str = "/api/v1/auth/keycloak";

// Same values as `sessions_refresh.rs`: both files run in the same test binary and process.
static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Same mounting as `main.rs`, with the Keycloak client registered only when given.
macro_rules! init_app {
    ($state:expr, $keycloak:expr) => {{
        let app = App::new().app_data($state.clone());
        let app = match $keycloak {
            Some(keycloak) => app.app_data(web::Data::new(keycloak)),
            None => app,
        };
        test::init_service(
            app.configure(public_config)
                .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
        )
        .await
    }};
}

async fn app_state() -> web::Data<AppState> {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    web::Data::new(AppState::new(String::new(), host.clone()).await)
}

fn body() -> Value {
    json!({
        "code": "auth-code-123",
        "redirect_uri": REDIRECT_URI,
        "code_verifier": "pkce-verifier-456",
        "nonce": "test-nonce",
        "device_info": "Firefox 142 on Ubuntu 24.04"
    })
}

/// Mock realm whose token endpoint returns an ID token for `email`, tweaked by `change`.
fn realm_signing_in(email: &str, change: impl FnOnce(&mut Value)) -> KeycloakMock {
    let mock = KeycloakMock::start();
    let mut claims = id_token_claims(mock.realm_url(), email);
    change(&mut claims);
    mock.respond_with_id_token(&sign(&claims, TestKey::A, TestKey::A.kid()));
    mock
}

// actix test services hold `Rc`s, so this future is `!Send`; tests run it on one thread anyway.
#[allow(clippy::future_not_send)]
async fn post_keycloak(
    state: &web::Data<AppState>,
    keycloak: Option<KeycloakClient>,
    payload: &Value,
) -> (StatusCode, String, Option<String>) {
    let app = init_app!(state, keycloak);
    let req = test::TestRequest::post()
        .uri(KEYCLOAK_PATH)
        .set_json(payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let authorization = resp
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(ToString::to_string);
    let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
    (status, body, authorization)
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_opens_core_session() {
    let state = app_state().await;
    let mock = realm_signing_in("alice@example.com", |_| {});

    let (status, body, authorization) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let jwt = authorization
        .as_deref()
        .and_then(|h| h.strip_prefix("Bearer "))
        .expect("Bearer JWT in the Authorization header")
        .to_string();
    assert_eq!(
        get_user_id_from_jwt(&jwt),
        Some(ALICE_ID.get().unwrap().to_string())
    );

    // The refresh token belongs to a real Core session, exactly like after a password login.
    let refresh_token = serde_json::from_str::<Value>(&body).unwrap()["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();
    let app = init_app!(state, None::<KeycloakClient>);
    let req = test::TestRequest::post()
        .uri(REFRESH_PATH)
        .set_json(json!({ "refresh_token": refresh_token }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_matches_email_case_insensitively() {
    let state = app_state().await;
    let mock = realm_signing_in("ALICE@example.com", |_| {});

    let (status, body, _) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_without_configuration_returns_503() {
    let state = app_state().await;

    let (status, body, _) = post_keycloak(&state, None, &body()).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "Keycloak sign-in is not configured.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_unknown_account_returns_403() {
    let state = app_state().await;
    let mock = realm_signing_in("stranger@danger.com", |_| {});

    let (status, body, authorization) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body,
        "No Mairie 360 account matches this Keycloak e-mail address."
    );
    assert_eq!(authorization, None);
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_archived_account_returns_403() {
    let state = app_state().await;
    let mock = realm_signing_in("bob@example.com", |_| {});

    let (status, body, _) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, "This Mairie 360 account is archived.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_unverified_email_returns_403() {
    let state = app_state().await;
    let mock = realm_signing_in("alice@example.com", |claims| {
        claims["email_verified"] = json!(false);
    });

    let (status, body, _) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, "The Keycloak account has no verified e-mail address.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_nonce_mismatch_returns_401() {
    let state = app_state().await;
    let mock = realm_signing_in("alice@example.com", |claims| {
        claims["nonce"] = json!("replayed-nonce");
    });

    let (status, body, _) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, "Invalid Keycloak ID token.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_rejected_code_returns_401() {
    let state = app_state().await;
    let mock = KeycloakMock::start();
    mock.respond_with(400, json!({ "error": "invalid_grant" }));

    let (status, body, _) = post_keycloak(&state, Some(mock.client()), &body()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, "Keycloak rejected the authorization code.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_unreachable_keycloak_returns_502() {
    let state = app_state().await;
    let client = KeycloakClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        None,
        "core-api",
        None,
    ));

    let (status, body, _) = post_keycloak(&state, Some(client), &body()).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body, "Keycloak is unavailable.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_missing_field_returns_400() {
    let state = app_state().await;
    let mock = realm_signing_in("alice@example.com", |_| {});
    let mut payload = body();
    payload.as_object_mut().unwrap().remove("code");

    let (status, _, _) = post_keycloak(&state, Some(mock.client()), &payload).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(mock.token_requests().is_empty());
}

#[tokio::test]
#[serial]
async fn test_keycloak_login_optional_fields_can_be_omitted() {
    let state = app_state().await;
    let mock = realm_signing_in("alice@example.com", |_| {});
    let mut payload = body();
    let fields = payload.as_object_mut().unwrap();
    fields.remove("code_verifier");
    fields.remove("nonce");

    let (status, body, _) = post_keycloak(&state, Some(mock.client()), &payload).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!mock.token_requests()[0].contains_key("code_verifier"));
}
