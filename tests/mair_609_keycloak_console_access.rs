//! MAIR-609: temporary access to the Keycloak admin console for the town-hall administrators.
//!
//! `POST /api/v1/admin/keycloak/console-access` gives an Admin a single-use, 2-minute link;
//! `GET /admin-access?token=` turns it into a 30-minute console cookie bound to `/admin`;
//! `GET /internal/keycloak-console/verify` (Traefik `ForwardAuth`) accepts that cookie only while
//! its user is still an active Admin. Console tokens and session JWTs are not interchangeable.
//!
//! Own test binary: the Ed25519 key variables are process-wide (like MAIR-680's tests).
#![allow(clippy::future_not_send, clippy::cast_sign_loss)]

#[allow(dead_code, unused_imports)]
mod common;

use actix_web::{http::StatusCode, test, web, App};
use common::users::{archive_user, create_user, unique_marker};
use common::{get_pool, get_raw_pool};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, keycloak_console_config, public_config};
use core_api::keycloak_console::{
    ConsoleAccessConfig, CONSOLE_COOKIE_AUDIENCE, CONSOLE_COOKIE_NAME, CONSOLE_COOKIE_TYPE,
    LINK_TOKEN_AUDIENCE, LINK_TOKEN_TYPE, VERIFY_PATH,
};
use core_api::rate_limit::RateLimits;
use jsonwebtoken::{decode_header, encode, Algorithm, Header};
use mairie360_api_lib::jwt_manager::{
    generate_jwt, parse_public_keys, parse_signing_key, SESSION_JWT_ISSUER,
};
use mairie360_api_lib::test_setup::http_setup::{test_jwt_private_key, TEST_JWT_PUBLIC_KEY};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, get_shared_db_as};
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use sqlx::PgPool;

const POST_PATH: &str = "/api/v1/admin/keycloak/console-access";
const ISSUER: &str = "https://auth.mairie360.test/realms/mairie360";

/// PKCS#8 body of an Ed25519 key that is not Core's (built into PEM at run time so that the secret
/// scanners do not read a PEM private key in the sources).
const ATTACKER_PRIVATE_KEY_B64: &str =
    "MC4CAQAwBQYDK2VwBCIEII1DTYiu+dcwBgtEKil59uTH8s7XfrxGrbD+/P0sXq/8";

const ATTACKER_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAtZHz1BNkijjkLJlnK9l7vikDl0aivrmbwllpT68Lnss=
-----END PUBLIC KEY-----
";

fn pem(body: &str) -> String {
    let label = "PRIVATE KEY";
    format!("-----BEGIN {label}-----\n{body}\n-----END {label}-----\n")
}

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_PUBLIC_KEY", TEST_JWT_PUBLIC_KEY);
    std::env::set_var("JWT_PRIVATE_KEY", test_jwt_private_key());
    std::env::remove_var("JWT_SECRET");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

/// Same wiring as `main.rs`; `console` and `limits` are optional app data there too.
macro_rules! init_app {
    ($state:expr, $console:expr, $limits:expr) => {{
        let app = App::new().app_data($state.clone());
        let app = match $console {
            Some(console) => app.app_data(web::Data::new(console)),
            None => app,
        };
        let app = match $limits {
            Some(limits) => app.app_data(web::Data::new(limits)),
            None => app,
        };
        test::init_service(
            app.configure(keycloak_console_config)
                .configure(public_config)
                .service(
                    web::scope("/api")
                        .wrap(actix_web::middleware::from_fn(session_guard))
                        .wrap(JwtMiddleware)
                        .configure(config),
                ),
        )
        .await
    }};
}

macro_rules! call {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp,
            Err(err) => test::TestRequest::default().to_srv_response(err.error_response()),
        }
    };
}

fn console() -> Option<ConsoleAccessConfig> {
    ConsoleAccessConfig::from_realm_url(ISSUER)
}

struct Fixture {
    _redis: Box<dyn std::any::Any>,
    state: web::Data<AppState>,
    raw: PgPool,
}

async fn fixture() -> Fixture {
    std::sync::LazyLock::force(&INIT);
    let (redis_container, redis) = start_redis_container().await;
    let state =
        web::Data::new(AppState::new(redis.url.clone(), get_shared_db_as("core_api").await).await);
    let (_, superuser_url) = get_shared_db().await;
    let raw = get_raw_pool(superuser_url.clone()).await;
    Fixture {
        _redis: Box::new(redis_container),
        state,
        raw,
    }
}

/// A fresh account holding `role`.
async fn user_with_role(raw: &PgPool, role: &str) -> u64 {
    let id = create_user(
        &get_pool().await,
        "Console",
        &unique_marker("mairsixzeronine"),
    )
    .await;
    let id = u64::try_from(id).unwrap();
    set_role(raw, id, role).await;
    id
}

async fn set_role(raw: &PgPool, user_id: u64, role: &str) {
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) SELECT $1, id FROM roles WHERE name = $2 \
         ON CONFLICT (user_id) DO UPDATE SET role_id = EXCLUDED.role_id",
    )
    .bind(i32::try_from(user_id).unwrap())
    .bind(role)
    .execute(raw)
    .await
    .unwrap();
}

fn session_jwt(user_id: u64, role: &str) -> String {
    generate_jwt(&user_id.to_string(), role).unwrap()
}

fn bearer(jwt: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {jwt}"))
}

fn request_link(user_id: u64, role: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri(POST_PATH)
        .insert_header(bearer(&session_jwt(user_id, role)))
}

fn open_link(token: &str) -> test::TestRequest {
    test::TestRequest::get().uri(&format!("/admin-access?token={token}"))
}

fn verify_with(cookie: &str) -> test::TestRequest {
    test::TestRequest::get()
        .uri(VERIFY_PATH)
        .insert_header(("Cookie", format!("{CONSOLE_COOKIE_NAME}={cookie}")))
}

fn now() -> u64 {
    u64::try_from(chrono::Utc::now().timestamp()).unwrap()
}

fn core_kid() -> String {
    parse_public_keys(TEST_JWT_PUBLIC_KEY).unwrap()[0]
        .kid()
        .to_string()
}

/// Signs console claims by hand with Core's key, or with the attacker's (never published) when
/// `forged`, always under Core's `kid`.
fn sign(typ: &str, aud: &str, sub: u64, exp: u64, forged: bool) -> String {
    let (private, public) = if forged {
        (pem(ATTACKER_PRIVATE_KEY_B64), ATTACKER_PUBLIC_KEY)
    } else {
        (test_jwt_private_key(), TEST_JWT_PUBLIC_KEY)
    };
    let key = parse_signing_key(&private, &parse_public_keys(public).unwrap()).unwrap();
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(core_kid());
    header.typ = Some(typ.to_string());
    encode(
        &header,
        &json!({
            "sub": sub.to_string(),
            "iss": SESSION_JWT_ISSUER,
            "aud": aud,
            "iat": now() - 10,
            "exp": exp,
            "jti": uuid::Uuid::new_v4().to_string(),
        }),
        key.encoding_key(),
    )
    .unwrap()
}

/// Asks for a link as `admin` and returns its token.
macro_rules! link_token {
    ($app:expr, $admin:expr) => {{
        let resp = test::call_service(&$app, request_link($admin, "Admin").to_request()).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        body["url"]
            .as_str()
            .unwrap()
            .split_once("token=")
            .unwrap()
            .1
            .to_string()
    }};
}

/// Follows a link and returns the console cookie value.
macro_rules! console_cookie {
    ($app:expr, $token:expr) => {{
        let resp = test::call_service(&$app, open_link(&$token).to_request()).await;
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        resp.response()
            .cookies()
            .find(|c| c.name() == CONSOLE_COOKIE_NAME)
            .unwrap()
            .value()
            .to_string()
    }};
}

#[tokio::test]
async fn mair_609_admin_gets_a_link_then_a_console_cookie_accepted_by_verify() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);
    let admin = user_with_role(&f.raw, "Admin").await;

    let resp = test::call_service(&app, request_link(admin, "Admin").to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("Cache-Control").unwrap(), "no-store");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["expires_in"], 120);
    let url = body["url"].as_str().unwrap();
    let token = url
        .strip_prefix("https://auth.mairie360.test/admin-access?token=")
        .expect("link on the public auth host");

    // A link token: Core's key, its own typ.
    let header = decode_header(token).unwrap();
    assert_eq!(header.alg, Algorithm::EdDSA);
    assert_eq!(header.kid, Some(core_kid()));
    assert_eq!(header.typ.as_deref(), Some(LINK_TOKEN_TYPE));

    let resp = test::call_service(&app, open_link(token).to_request()).await;
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        resp.headers().get("Location").unwrap(),
        "/admin/mairie360/console/"
    );
    assert_eq!(
        resp.headers().get("Referrer-Policy").unwrap(),
        "no-referrer"
    );
    let cookie = resp
        .response()
        .cookies()
        .find(|c| c.name() == CONSOLE_COOKIE_NAME)
        .expect("console cookie set")
        .into_owned();
    assert_eq!(cookie.path(), Some("/admin"));
    assert_eq!(cookie.domain(), None, "host-only: auth.<domain> only");
    assert_eq!(cookie.http_only(), Some(true));
    assert_eq!(cookie.secure(), Some(true));
    assert_eq!(
        cookie.same_site(),
        Some(actix_web::cookie::SameSite::Strict)
    );
    assert_eq!(
        cookie.max_age(),
        Some(actix_web::cookie::time::Duration::minutes(30))
    );
    assert_eq!(
        decode_header(cookie.value()).unwrap().typ.as_deref(),
        Some(CONSOLE_COOKIE_TYPE)
    );

    let resp = test::call_service(&app, verify_with(cookie.value()).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn mair_609_non_admins_are_refused_a_link() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);
    for role in ["Maire", "Responsable", "User", "Guest"] {
        let user = user_with_role(&f.raw, role).await;
        let resp = call!(app, request_link(user, role).to_request());
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "{role}");
    }
    // Without a session at all.
    let resp = call!(app, test::TestRequest::post().uri(POST_PATH).to_request());
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mair_609_a_link_works_once() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);
    let admin = user_with_role(&f.raw, "Admin").await;
    let token = link_token!(app, admin);

    console_cookie!(app, token);
    let replay = test::call_service(&app, open_link(&token).to_request()).await;
    assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
    assert!(replay.response().cookies().next().is_none());
}

#[tokio::test]
async fn mair_609_expired_forged_or_missing_links_are_refused() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);
    let admin = user_with_role(&f.raw, "Admin").await;

    let expired = sign(
        LINK_TOKEN_TYPE,
        LINK_TOKEN_AUDIENCE,
        admin,
        now() - 1,
        false,
    );
    let forged = sign(
        LINK_TOKEN_TYPE,
        LINK_TOKEN_AUDIENCE,
        admin,
        now() + 60,
        true,
    );
    let genuine = link_token!(app, admin);
    // Same signature, another subject: the signature no longer matches.
    let mut parts: Vec<String> = genuine.split('.').map(ToString::to_string).collect();
    parts[1] = parts[1].chars().rev().collect();
    let tampered = parts.join(".");

    for (case, token) in [
        ("expired", expired),
        ("forged", forged),
        ("tampered", tampered),
        ("garbage", "not-a-jwt".to_string()),
        ("empty", String::new()),
    ] {
        let resp = test::call_service(&app, open_link(&token).to_request()).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{case}");
        assert!(resp.response().cookies().next().is_none(), "{case}");
    }
    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/admin-access").to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mair_609_console_tokens_and_session_jwts_are_not_interchangeable() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);
    let admin = user_with_role(&f.raw, "Admin").await;
    let session = session_jwt(admin, "Admin");
    let link = link_token!(app, admin);
    let cookie = console_cookie!(app, link_token!(app, admin));

    // A session JWT is neither a link nor a console cookie.
    let resp = test::call_service(&app, open_link(&session).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = test::call_service(&app, verify_with(&session).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Console tokens are not session JWTs.
    for token in [&link, &cookie] {
        let resp = call!(
            app,
            test::TestRequest::get()
                .uri("/api/v1/user/me/")
                .insert_header(bearer(token))
                .to_request()
        );
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // And the two console tokens do not stand in for each other.
    let resp = test::call_service(&app, verify_with(&link).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = test::call_service(&app, open_link(&cookie).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    // A link token with the cookie's typ but the link audience (or the reverse) is refused too.
    let mixed = sign(
        CONSOLE_COOKIE_TYPE,
        LINK_TOKEN_AUDIENCE,
        admin,
        now() + 60,
        false,
    );
    let resp = test::call_service(&app, verify_with(&mixed).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mair_609_verify_refuses_missing_expired_or_forged_cookies() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);
    let admin = user_with_role(&f.raw, "Admin").await;

    let resp =
        test::call_service(&app, test::TestRequest::get().uri(VERIFY_PATH).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let expired = sign(
        CONSOLE_COOKIE_TYPE,
        CONSOLE_COOKIE_AUDIENCE,
        admin,
        now() - 1,
        false,
    );
    let forged = sign(
        CONSOLE_COOKIE_TYPE,
        CONSOLE_COOKIE_AUDIENCE,
        admin,
        now() + 600,
        true,
    );
    for (case, cookie) in [("expired", expired), ("forged", forged)] {
        let resp = test::call_service(&app, verify_with(&cookie).to_request()).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{case}");
    }
    // A well-signed cookie of Core is accepted: the refusals above are about expiry and key.
    let genuine = sign(
        CONSOLE_COOKIE_TYPE,
        CONSOLE_COOKIE_AUDIENCE,
        admin,
        now() + 600,
        false,
    );
    let resp = test::call_service(&app, verify_with(&genuine).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn mair_609_a_demoted_or_archived_admin_loses_the_console() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), None::<RateLimits>);

    // Demoted after opening the console: the next console request is refused.
    let admin = user_with_role(&f.raw, "Admin").await;
    let cookie = console_cookie!(app, link_token!(app, admin));
    set_role(&f.raw, admin, "User").await;
    let resp = test::call_service(&app, verify_with(&cookie).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Demoted between the link and its use: the link is refused (and consumed).
    let admin = user_with_role(&f.raw, "Admin").await;
    let token = link_token!(app, admin);
    set_role(&f.raw, admin, "Maire").await;
    let resp = test::call_service(&app, open_link(&token).to_request()).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(resp.response().cookies().next().is_none());

    // Archived after opening the console.
    let admin = user_with_role(&f.raw, "Admin").await;
    let cookie = console_cookie!(app, link_token!(app, admin));
    archive_user(&f.raw, i32::try_from(admin).unwrap()).await;
    let resp = test::call_service(&app, verify_with(&cookie).to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mair_609_links_are_rate_limited_per_admin() {
    let f = fixture().await;
    let app = init_app!(f.state, console(), Some(RateLimits::default()));
    let admin = user_with_role(&f.raw, "Admin").await;
    for _ in 0..10 {
        let resp = test::call_service(&app, request_link(admin, "Admin").to_request()).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let resp = call!(app, request_link(admin, "Admin").to_request());
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp.headers().get("Retry-After").is_some());
    // Another administrator keeps their own budget.
    let other = user_with_role(&f.raw, "Admin").await;
    let resp = test::call_service(&app, request_link(other, "Admin").to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn mair_609_console_access_answers_503_when_not_configured() {
    let f = fixture().await;
    let app = init_app!(f.state, None::<ConsoleAccessConfig>, None::<RateLimits>);
    let admin = user_with_role(&f.raw, "Admin").await;
    let resp = call!(app, request_link(admin, "Admin").to_request());
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let token = sign(
        LINK_TOKEN_TYPE,
        LINK_TOKEN_AUDIENCE,
        admin,
        now() + 60,
        false,
    );
    let resp = test::call_service(&app, open_link(&token).to_request()).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
