//! Passkeys (MAIR-505): registration and management under `/api/v1/user/me/passkeys/`, sign-in
//! under `/api/v1/auth/passkey`, driven by a software authenticator (no browser).

use crate::common::get_raw_pool;
use crate::common::passkey::{Authenticator, ORIGIN, RP_ID};
use crate::common::users::{archive_user, create_user, unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::rate_limit::RateLimits;
use core_api::webauthn::WebauthnConfig;
use mairie360_api_lib::jwt_manager::{generate_jwt, get_user_id_from_jwt};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use url::Url;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Webauthn,
};

const OPTIONS_PATH: &str = "/api/v1/user/me/passkeys/options";
const PASSKEYS_PATH: &str = "/api/v1/user/me/passkeys/";
const LOGIN_OPTIONS_PATH: &str = "/api/v1/auth/passkey/options";
const LOGIN_PATH: &str = "/api/v1/auth/passkey";

// Same values as `sessions_refresh.rs`: both files run in the same test binary and process.
static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

fn relying_party() -> Webauthn {
    WebauthnConfig::new(RP_ID, None, vec![Url::parse(ORIGIN).unwrap()])
        .build()
        .unwrap()
}

/// Same mounting as `main.rs`, with the relying party registered only when asked.
macro_rules! init_app {
    ($state:expr, $webauthn:expr) => {{
        let app = App::new()
            .app_data($state.clone())
            .app_data(web::Data::new(RateLimits::default()));
        let app = match $webauthn {
            Some(webauthn) => app.app_data(web::Data::new(webauthn)),
            None => app,
        };
        test::init_service(
            app.configure(public_config).service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
        )
        .await
    }};
}

/// Status of a call, whether a handler answered or a middleware refused it.
macro_rules! status {
    ($app:expr, $req:expr) => {
        match test::try_call_service(&$app, $req).await {
            Ok(resp) => resp.status(),
            Err(err) => err.as_response_error().status_code(),
        }
    };
}

/// Shared database and a Redis of its own (the pending ceremonies live there).
struct Stack {
    _redis: testcontainers::ContainerAsync<testcontainers::GenericImage>,
    state: web::Data<AppState>,
    /// Postgres URL of the shared database, for the raw fixtures.
    host: String,
}

async fn stack() -> Stack {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let (redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), host.clone()).await);
    Stack {
        _redis: redis,
        state,
        host: host.clone(),
    }
}

fn bearer(user_id: i32) -> (&'static str, String) {
    let token = generate_jwt(&user_id.to_string(), "user").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

async fn body_json(resp: actix_web::dev::ServiceResponse) -> Value {
    serde_json::from_slice(&test::read_body(resp).await).unwrap()
}

// actix test services hold `Rc`s: these futures are `!Send`, tests run them on one thread anyway.
#[allow(clippy::future_not_send)]
async fn registration_options<S>(app: &S, user_id: i32) -> (String, CreationChallengeResponse)
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let resp = test::call_service(
        app,
        test::TestRequest::post()
            .uri(OPTIONS_PATH)
            .insert_header(bearer(user_id))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    (
        body["challenge_id"].as_str().unwrap().to_string(),
        serde_json::from_value(body["public_key"].clone()).unwrap(),
    )
}

#[allow(clippy::future_not_send)]
async fn register<S>(
    app: &S,
    user_id: i32,
    challenge_id: &str,
    credential: &RegisterPublicKeyCredential,
    label: &str,
) -> actix_web::dev::ServiceResponse
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    test::call_service(
        app,
        test::TestRequest::post()
            .uri(PASSKEYS_PATH)
            .insert_header(bearer(user_id))
            .set_json(json!({
                "challenge_id": challenge_id,
                "label": label,
                "credential": credential,
            }))
            .to_request(),
    )
    .await
}

/// Registers a passkey for `user_id` and returns its id in the list.
#[allow(clippy::future_not_send)]
async fn register_passkey<S>(app: &S, authenticator: &mut Authenticator, user_id: i32) -> i64
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let (challenge_id, options) = registration_options(app, user_id).await;
    let credential = authenticator.register(ORIGIN, options);
    let resp = register(app, user_id, &challenge_id, &credential, "Clé de test").await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["label"], "Clé de test");
    assert!(body["last_used_at"].is_null());
    body["id"].as_i64().unwrap()
}

#[allow(clippy::future_not_send)]
async fn login_options<S>(app: &S) -> (String, RequestChallengeResponse)
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let resp = test::call_service(
        app,
        test::TestRequest::post()
            .uri(LOGIN_OPTIONS_PATH)
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let options: RequestChallengeResponse =
        serde_json::from_value(body["public_key"].clone()).unwrap();
    assert!(
        options.public_key.allow_credentials.is_empty(),
        "a discoverable sign-in names no credential"
    );
    (body["challenge_id"].as_str().unwrap().to_string(), options)
}

fn login_request(challenge_id: &str, credential: &PublicKeyCredential) -> actix_http::Request {
    test::TestRequest::post()
        .uri(LOGIN_PATH)
        .set_json(json!({
            "challenge_id": challenge_id,
            "credential": credential,
            "device_info": "soft passkey",
        }))
        .to_request()
}

/// Opens a sign-in ceremony and answers it with the authenticator's last passkey.
#[allow(clippy::future_not_send)]
async fn sign_in<S>(app: &S, authenticator: &mut Authenticator) -> actix_web::dev::ServiceResponse
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let (challenge_id, options) = login_options(app).await;
    let credential_id = authenticator.last_credential_id().to_vec();
    let assertion = authenticator.authenticate(ORIGIN, options, &credential_id);
    test::call_service(app, login_request(&challenge_id, &assertion)).await
}

#[tokio::test]
#[serial]
async fn a_registered_passkey_signs_in_and_can_be_deleted() {
    let stack = stack().await;
    let raw = get_raw_pool(stack.host.clone()).await;
    let user_id = create_user(stack.state.get_smart_db(), "Pass", &unique_marker("key")).await;
    let app = init_app!(stack.state, Some(relying_party()));
    let mut authenticator = Authenticator::new();

    // No passkey yet.
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(PASSKEYS_PATH)
            .insert_header(bearer(user_id))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await["passkeys"], json!([]));

    let passkey_id = register_passkey(&app, &mut authenticator, user_id).await;

    // The options of a second registration exclude the first passkey.
    let (_, options) = registration_options(&app, user_id).await;
    let excluded: Vec<Vec<u8>> = options
        .public_key
        .exclude_credentials
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.id.to_vec())
        .collect();
    assert_eq!(excluded, vec![authenticator.last_credential_id().to_vec()]);
    assert_eq!(
        options.public_key.user.name,
        sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&raw)
            .await
            .unwrap()
    );

    // Sign in: a Core session, like after a password login.
    let resp = sign_in(&app, &mut authenticator).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let jwt = resp
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .expect("Bearer JWT in the Authorization header")
        .to_string();
    assert_eq!(get_user_id_from_jwt(&jwt), Some(user_id.to_string()));
    let body = body_json(resp).await;
    assert!(body["refresh_token"]
        .as_str()
        .is_some_and(|t| !t.is_empty()));

    // The sign-in is recorded on the passkey.
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(PASSKEYS_PATH)
            .insert_header(bearer(user_id))
            .to_request(),
    )
    .await;
    let passkeys = body_json(resp).await["passkeys"].clone();
    assert_eq!(passkeys.as_array().unwrap().len(), 1);
    assert_eq!(passkeys[0]["id"].as_i64(), Some(passkey_id));
    assert!(passkeys[0]["last_used_at"].is_string());
    assert!(
        passkeys[0].get("passkey").is_none() && passkeys[0].get("credential_id").is_none(),
        "the public key never leaves Core"
    );

    // Delete it: it cannot sign in any more.
    let resp = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("{PASSKEYS_PATH}{passkey_id}/"))
            .insert_header(bearer(user_id))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = sign_in(&app, &mut authenticator).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("{PASSKEYS_PATH}{passkey_id}/"))
            .insert_header(bearer(user_id))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn a_sign_in_challenge_is_single_use() {
    let stack = stack().await;
    let user_id = create_user(stack.state.get_smart_db(), "Once", &unique_marker("key")).await;
    let app = init_app!(stack.state, Some(relying_party()));
    let mut authenticator = Authenticator::new();
    register_passkey(&app, &mut authenticator, user_id).await;

    let (challenge_id, options) = login_options(&app).await;
    let credential_id = authenticator.last_credential_id().to_vec();
    let assertion = authenticator.authenticate(ORIGIN, options, &credential_id);
    let first = test::call_service(&app, login_request(&challenge_id, &assertion)).await;
    assert_eq!(first.status(), StatusCode::OK);
    let replay = test::call_service(&app, login_request(&challenge_id, &assertion)).await;
    assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);

    // A challenge never issued is refused the same way.
    let (_, options) = login_options(&app).await;
    let assertion = authenticator.authenticate(ORIGIN, options, &credential_id);
    let unknown = test::call_service(
        &app,
        login_request(&uuid::Uuid::new_v4().to_string(), &assertion),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn an_assertion_for_another_origin_is_refused() {
    let stack = stack().await;
    let user_id = create_user(stack.state.get_smart_db(), "Origin", &unique_marker("key")).await;
    let app = init_app!(stack.state, Some(relying_party()));
    let mut authenticator = Authenticator::new();
    register_passkey(&app, &mut authenticator, user_id).await;

    let (challenge_id, options) = login_options(&app).await;
    let credential_id = authenticator.last_credential_id().to_vec();
    let assertion = authenticator.authenticate("https://evil.example", options, &credential_id);
    let resp = test::call_service(&app, login_request(&challenge_id, &assertion)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn an_archived_account_cannot_sign_in_with_its_passkey() {
    let stack = stack().await;
    let raw = get_raw_pool(stack.host.clone()).await;
    let user_id = create_user(
        stack.state.get_smart_db(),
        "Archived",
        &unique_marker("key"),
    )
    .await;
    let app = init_app!(stack.state, Some(relying_party()));
    let mut authenticator = Authenticator::new();
    register_passkey(&app, &mut authenticator, user_id).await;

    archive_user(&raw, user_id).await;

    let resp = sign_in(&app, &mut authenticator).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn a_registration_challenge_belongs_to_the_account_that_opened_it() {
    let stack = stack().await;
    let owner = create_user(stack.state.get_smart_db(), "Owner", &unique_marker("key")).await;
    let other = create_user(stack.state.get_smart_db(), "Other", &unique_marker("key")).await;
    let app = init_app!(stack.state, Some(relying_party()));
    let mut authenticator = Authenticator::new();

    let (challenge_id, options) = registration_options(&app, owner).await;
    let credential = authenticator.register(ORIGIN, options);
    let resp = register(&app, other, &challenge_id, &credential, "Volée").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    // The challenge is spent: the owner cannot use it either.
    let resp = register(&app, owner, &challenge_id, &credential, "Trop tard").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // A fresh ceremony answered for another origin is refused too.
    let (challenge_id, options) = registration_options(&app, owner).await;
    let credential = authenticator.register("https://evil.example", options);
    let resp = register(&app, owner, &challenge_id, &credential, "Ailleurs").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // The label is validated before anything else.
    let (challenge_id, options) = registration_options(&app, owner).await;
    let credential = authenticator.register(ORIGIN, options);
    let resp = register(&app, owner, &challenge_id, &credential, "   ").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert!(String::from_utf8(test::read_body(resp).await.to_vec())
        .unwrap()
        .contains("`label`"));
}

#[tokio::test]
#[serial]
async fn a_passkey_of_another_account_cannot_be_deleted() {
    let stack = stack().await;
    let owner = create_user(stack.state.get_smart_db(), "Owner", &unique_marker("key")).await;
    let other = create_user(stack.state.get_smart_db(), "Other", &unique_marker("key")).await;
    let app = init_app!(stack.state, Some(relying_party()));
    let mut authenticator = Authenticator::new();
    let passkey_id = register_passkey(&app, &mut authenticator, owner).await;

    let resp = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("{PASSKEYS_PATH}{passkey_id}/"))
            .insert_header(bearer(other))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Still there for its owner.
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(PASSKEYS_PATH)
            .insert_header(bearer(owner))
            .to_request(),
    )
    .await;
    assert_eq!(
        body_json(resp).await["passkeys"].as_array().unwrap().len(),
        1
    );
}

#[tokio::test]
#[serial]
async fn passkeys_answer_503_when_not_configured() {
    let stack = stack().await;
    let user_id = create_user(stack.state.get_smart_db(), "Off", &unique_marker("key")).await;
    let app = init_app!(stack.state, None::<Webauthn>);

    for req in [
        test::TestRequest::post().uri(LOGIN_OPTIONS_PATH),
        test::TestRequest::post().uri(LOGIN_PATH).set_json(json!({
            "challenge_id": uuid::Uuid::new_v4(),
            "credential": {
                "id": "AAAAAAAAAAAAAAAAAAAAAA",
                "rawId": "AAAAAAAAAAAAAAAAAAAAAA",
                "type": "public-key",
                "response": {
                    "authenticatorData": "AA",
                    "clientDataJSON": "AA",
                    "signature": "AA",
                    "userHandle": null
                },
                "clientExtensionResults": {}
            },
            "device_info": "none"
        })),
        test::TestRequest::post()
            .uri(OPTIONS_PATH)
            .insert_header(bearer(user_id)),
    ] {
        let status = status!(app, req.to_request());
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    // Listing and deleting need no relying party.
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(PASSKEYS_PATH)
            .insert_header(bearer(user_id))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn the_management_routes_require_a_jwt() {
    let stack = stack().await;
    let app = init_app!(stack.state, Some(relying_party()));

    for req in [
        test::TestRequest::post().uri(OPTIONS_PATH),
        test::TestRequest::get().uri(PASSKEYS_PATH),
        test::TestRequest::post()
            .uri(PASSKEYS_PATH)
            .set_json(json!({})),
        test::TestRequest::delete().uri(&format!("{PASSKEYS_PATH}1/")),
    ] {
        let status = status!(app, req.to_request());
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
