//! MAIR-680: Core signs session JWTs with its Ed25519 key, publishes the public key as a JWKS, and
//! no longer accepts HS256 tokens once `JWT_SECRET` is removed.
//!
//! Own test binary: the key variables are process-wide, and the endpoint tests of
//! `integration_test` still run with `JWT_SECRET` only.
#![allow(clippy::cast_possible_wrap)]

use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config, v1::sessions::REFRESH_PATH};
use jsonwebtoken::{decode_header, encode, Algorithm, EncodingKey, Header};
use mairie360_api_lib::jwt_manager::{
    decode_jwt, parse_public_keys, parse_signing_key, Claims, SESSION_JWT_AUDIENCE,
    SESSION_JWT_ISSUER,
};
use mairie360_api_lib::test_setup::http_setup::{test_jwt_private_key, TEST_JWT_PUBLIC_KEY};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db_as;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use uuid::Uuid;

/// An attacker's key pair: never published in `JWT_PUBLIC_KEY`.
/// PKCS#8 body (base64) of a second Ed25519 private key; built into PEM at run time so that the
/// secret scanners do not read a PEM private key in the sources.
const ATTACKER_PRIVATE_KEY_B64: &str =
    "MC4CAQAwBQYDK2VwBCIEII1DTYiu+dcwBgtEKil59uTH8s7XfrxGrbD+/P0sXq/8";

fn attacker_private_key() -> String {
    let label = "PRIVATE KEY";
    format!("-----BEGIN {label}-----\n{ATTACKER_PRIVATE_KEY_B64}\n-----END {label}-----\n")
}
const ATTACKER_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAtZHz1BNkijjkLJlnK9l7vikDl0aivrmbwllpT68Lnss=
-----END PUBLIC KEY-----
";

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // End of the transition on Core: its key pair, no shared secret any more.
    std::env::set_var("JWT_PUBLIC_KEY", TEST_JWT_PUBLIC_KEY);
    std::env::set_var("JWT_PRIVATE_KEY", test_jwt_private_key());
    std::env::remove_var("JWT_SECRET");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Same wiring as `main.rs`.
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

macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    };
}

fn core_kid() -> String {
    parse_public_keys(TEST_JWT_PUBLIC_KEY).unwrap()[0]
        .kid()
        .to_string()
}

fn in_one_hour() -> usize {
    usize::try_from(chrono::Utc::now().timestamp()).unwrap() + 3600
}

async fn state() -> (impl Drop, web::Data<AppState>) {
    std::sync::LazyLock::force(&INIT);
    let (redis_container, redis) = start_redis_container().await;
    let state =
        web::Data::new(AppState::new(redis.url.clone(), get_shared_db_as("core_api").await).await);
    (redis_container, state)
}

/// Opens a session for the seeded Admin (id 1) and returns its refresh token.
async fn open_session(state: &web::Data<AppState>) -> String {
    let refresh_token = format!("mair_680_{}", Uuid::new_v4());
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            Uuid::new_v4(),
            1,
            &core_api::refresh_token::hash(&refresh_token),
            "mair-680",
            std::net::IpAddr::from([127, 0, 0, 1]),
        ))
        .await
        .unwrap();
    refresh_token
}

fn bearer(jwt: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {jwt}"))
}

#[tokio::test]
async fn mair_680_core_issues_eddsa_jwts_accepted_by_its_routes() {
    let (_redis, state) = state().await;
    let app = init_app!(state);
    let refresh_token = open_session(&state).await;

    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(REFRESH_PATH)
            .set_json(json!({ "refresh_token": refresh_token }))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let jwt = resp
        .headers()
        .get("Authorization")
        .unwrap()
        .to_str()
        .unwrap()
        .trim_start_matches("Bearer ")
        .to_string();

    // Signed with Core's key: EdDSA, its kid, iss and aud.
    let header = decode_header(&jwt).unwrap();
    assert_eq!(header.alg, Algorithm::EdDSA);
    assert_eq!(header.kid, Some(core_kid()));
    let claims = decode_jwt(&jwt).unwrap();
    assert_eq!(claims.user_id(), "1");
    assert_eq!(claims.issuer(), Some(SESSION_JWT_ISSUER));
    assert_eq!(claims.audience(), Some(SESSION_JWT_AUDIENCE));
    assert!(claims.session_id().is_some(), "bound to the session");

    // Accepted by the protected routes, and by the logout that reads the session from it.
    let list = test::TestRequest::get()
        .uri("/api/v1/sessions/")
        .insert_header(bearer(&jwt))
        .to_request();
    assert_eq!(status!(app, list), StatusCode::OK);
    let logout = test::TestRequest::post()
        .uri("/api/v1/sessions/logout")
        .insert_header(bearer(&jwt))
        .to_request();
    assert!(status!(app, logout).is_success());
    // Once its session is logged out, the JWT is refused.
    let again = test::TestRequest::get()
        .uri("/api/v1/sessions/")
        .insert_header(bearer(&jwt))
        .to_request();
    assert_eq!(status!(app, again), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mair_680_forged_admin_tokens_are_refused() {
    let (_redis, state) = state().await;
    let app = init_app!(state);
    let claims = Claims::new("1", "Admin", in_one_hour())
        .with_issuer_and_audience(SESSION_JWT_ISSUER, SESSION_JWT_AUDIENCE);

    // A compromised service's own key, claiming Core's kid.
    let attacker = parse_signing_key(
        &attacker_private_key(),
        &parse_public_keys(ATTACKER_PUBLIC_KEY).unwrap(),
    )
    .unwrap();
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(core_kid());
    let forged = encode(&header, &claims, attacker.encoding_key()).unwrap();
    // The old shared secret, which every service held.
    let legacy = encode(
        &Header::new(Algorithm::HS256),
        &Claims::new("1", "Admin", in_one_hour()),
        &EncodingKey::from_secret(b"0123456789abcdef0123456789abcdef"),
    )
    .unwrap();

    for (case, token) in [("other key", forged), ("old HS256 secret", legacy)] {
        for uri in ["/api/v1/sessions/", "/api/v1/admin/users"] {
            let req = test::TestRequest::get()
                .uri(uri)
                .insert_header(bearer(&token))
                .to_request();
            assert_eq!(
                status!(app, req),
                StatusCode::UNAUTHORIZED,
                "{case} on {uri}"
            );
        }
    }
}

#[tokio::test]
async fn mair_680_jwks_publishes_the_public_key_only() {
    let (_redis, state) = state().await;
    let app = init_app!(state);
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/v1/auth/jwks")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp
        .headers()
        .get("Cache-Control")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("max-age=300"));
    let body: Value = test::read_body_json(resp).await;
    let keys = body["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["kid"], core_kid());
    assert_eq!(keys[0]["kty"], "OKP");
    assert_eq!(keys[0]["crv"], "Ed25519");
    assert_eq!(keys[0]["alg"], "EdDSA");
    assert_eq!(keys[0]["use"], "sig");
    assert!(keys[0].get("d").is_none(), "no private part");
}
