//! Rate limits of the public authentication routes (MAIR-390, MAIR-474): every budget of
//! [`RateLimits`] is pinned on the route that spends it, with its production value.
//!
//! The load tests run with `RATE_LIMIT_ENABLED=false`, so these tests are the only check that the
//! limits apply. `login_is_rate_limited_per_email` (in `auth_hardening`) covers the per-account
//! login budget.

use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use core_api::keycloak::{KeycloakClient, KeycloakConfig};
use core_api::rate_limit::RateLimits;
use mairie360_api_lib::{
    security::JwtMiddleware, state::AppState, test_setup::queries_setup::get_shared_db,
};
use serde_json::{json, Value};
use serial_test::serial;
use std::net::SocketAddr;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as `auth_hardening`: both modules run in the same test binary.
    std::env::set_var("JWT_SECRET", "auth_hardening_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const LOGIN_PATH: &str = "/api/v1/auth/login";
const KEYCLOAK_PATH: &str = "/api/v1/auth/keycloak";
const FORGOT_PASSWORD_PATH: &str = "/api/v1/auth/forgot_password";
const RESET_PASSWORD_PATH: &str = "/api/v1/auth/reset_password";
const FORCE_CHANGE_PASSWORD_PATH: &str = "/api/v1/auth/force_change_password";

/// Public address of the client: not a trusted proxy, so its forwarding headers are ignored.
const CLIENT: &str = "203.0.113.7:40000";
const OTHER_CLIENT: &str = "203.0.113.8:40000";

/// Same wiring as `main.rs`, with the production rate limits and a Keycloak client that is never
/// reached (the limit refuses the request first).
macro_rules! init_app {
    ($state:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .app_data(web::Data::new(RateLimits::default()))
                .app_data(web::Data::new(KeycloakClient::new(KeycloakConfig::new(
                    "http://127.0.0.1:9/realms/test",
                    None,
                    "core-api",
                    None,
                ))))
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

async fn setup() -> web::Data<AppState> {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    web::Data::new(AppState::new("redis://127.0.0.1:6379".to_string(), host.clone()).await)
}

fn unique_email() -> String {
    format!("rate.{}@example.com", uuid::Uuid::new_v4())
}

/// `POST path` with a JSON body, sent from `peer` with a forged `X-Forwarded-For`.
fn post(path: &str, body: Value, peer: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri(path)
        .peer_addr(peer.parse::<SocketAddr>().unwrap())
        .insert_header(("X-Forwarded-For", format!("198.51.100.{}", rand_octet())))
        .set_json(body)
}

/// A different forged address on every request: the limiter must not trust it.
fn rand_octet() -> u8 {
    uuid::Uuid::new_v4().as_bytes()[0]
}

fn login_body(email: &str) -> Value {
    json!({ "email": email, "password": "Wrong!Pass123", "device_info": "test" })
}

fn keycloak_body() -> Value {
    json!({
        "code": "code",
        "redirect_uri": "http://localhost/callback",
        "device_info": "test"
    })
}

/// Valid for both routes (`force_change_password` ignores `device_info`), with a token nobody
/// was given.
fn password_token_body() -> Value {
    json!({
        "token": uuid::Uuid::new_v4().to_string(),
        "new_password": "Correct!Pass123",
        "device_info": "test"
    })
}

/// Sends `count` requests built by `request` and asserts each one reached the handler without
/// being rate limited. A `400` is refused by the body validation before the handler runs, so it
/// would spend nothing and let the test pass for the wrong reason.
macro_rules! spend {
    ($app:expr, $count:expr, $request:expr) => {
        for attempt in 0..$count {
            let status = test::call_service(&$app, $request(attempt).to_request())
                .await
                .status();
            assert!(
                status != StatusCode::TOO_MANY_REQUESTS && status != StatusCode::BAD_REQUEST,
                "attempt {attempt} answered {status} before the budget was spent"
            );
        }
    };
}

/// Asserts `request` gets `429` with a `Retry-After` header.
macro_rules! assert_limited {
    ($app:expr, $request:expr) => {
        let resp = test::call_service(&$app, $request.to_request()).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after: u64 = resp
            .headers()
            .get("retry-after")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(retry_after > 0);
    };
}

#[tokio::test]
#[serial]
async fn login_is_rate_limited_per_address() {
    let state = setup().await;
    let app = init_app!(state);

    // 30 per minute, each attempt on another account so the per-account budget is not the one hit.
    spend!(app, 30, |_| post(
        LOGIN_PATH,
        login_body(&unique_email()),
        CLIENT
    ));
    assert_limited!(app, post(LOGIN_PATH, login_body(&unique_email()), CLIENT));

    // Another client keeps its own budget.
    let resp = test::call_service(
        &app,
        post(LOGIN_PATH, login_body(&unique_email()), OTHER_CLIENT).to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn keycloak_login_shares_the_login_budget() {
    let state = setup().await;
    let app = init_app!(state);

    spend!(app, 30, |_| post(
        LOGIN_PATH,
        login_body(&unique_email()),
        CLIENT
    ));
    assert_limited!(app, post(KEYCLOAK_PATH, keycloak_body(), CLIENT));
}

#[tokio::test]
#[serial]
async fn forgot_password_is_rate_limited_per_email() {
    let state = setup().await;
    let app = init_app!(state);
    let email = unique_email();

    // 3 per hour and per address, whatever the client.
    spend!(app, 3, |attempt| post(
        FORGOT_PASSWORD_PATH,
        json!({ "email": email }),
        if attempt % 2 == 0 {
            CLIENT
        } else {
            OTHER_CLIENT
        }
    ));
    // Another case of the same address does not open a new budget.
    assert_limited!(
        app,
        post(
            FORGOT_PASSWORD_PATH,
            json!({ "email": email.to_uppercase() }),
            OTHER_CLIENT
        )
    );

    let resp = test::call_service(
        &app,
        post(
            FORGOT_PASSWORD_PATH,
            json!({ "email": unique_email() }),
            CLIENT,
        )
        .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn forgot_password_is_rate_limited_per_address() {
    let state = setup().await;
    let app = init_app!(state);

    // 10 per 15 minutes, each on another e-mail.
    spend!(app, 10, |_| post(
        FORGOT_PASSWORD_PATH,
        json!({ "email": unique_email() }),
        CLIENT
    ));
    assert_limited!(
        app,
        post(
            FORGOT_PASSWORD_PATH,
            json!({ "email": unique_email() }),
            CLIENT
        )
    );
}

#[tokio::test]
#[serial]
async fn password_token_routes_share_one_budget() {
    let state = setup().await;
    let app = init_app!(state);

    // 10 per 15 minutes across reset_password and force_change_password: guessing tokens on one
    // route does not reopen the other.
    spend!(app, 10, |attempt| post(
        if attempt % 2 == 0 {
            RESET_PASSWORD_PATH
        } else {
            FORCE_CHANGE_PASSWORD_PATH
        },
        password_token_body(),
        CLIENT
    ));
    assert_limited!(
        app,
        post(RESET_PASSWORD_PATH, password_token_body(), CLIENT)
    );
    assert_limited!(
        app,
        post(FORCE_CHANGE_PASSWORD_PATH, password_token_body(), CLIENT)
    );
}

#[tokio::test]
#[serial]
async fn refresh_is_rate_limited_per_address() {
    let state = setup().await;
    let app = init_app!(state);
    let body = || json!({ "refresh_token": uuid::Uuid::new_v4().to_string() });

    // 60 per minute.
    spend!(app, 60, |_| post(REFRESH_PATH, body(), CLIENT));
    assert_limited!(app, post(REFRESH_PATH, body(), CLIENT));
}
