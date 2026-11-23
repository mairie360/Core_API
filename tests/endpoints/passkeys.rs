//! MAIR-810: passkeys (WebAuthn) managed by Core, end to end on the real routes with a software
//! authenticator: registration, discoverable sign-in, replay and clone protections, ownership,
//! archived accounts, the instance policy and the security e-mails.

use crate::common::fake_resend;
use crate::common::{core_db_url, get_raw_pool};
use actix_web::{http::StatusCode, test, web, App};
use base64::Engine as _;
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::passkeys::Passkeys;
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::get_shared_redis;
use mairie360_api_lib::{password::hash_password, security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse, Url};

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";
const RP_ID: &str = "localhost";
const ORIGIN: &str = "https://localhost";

/// Same wiring as `main.rs`; `$passkeys` is an `Option<web::Data<Passkeys>>`.
macro_rules! init_app {
    ($state:expr, $passkeys:expr) => {{
        let app = App::new()
            .app_data($state.clone())
            .app_data(fake_resend::account_emails());
        let app = match &$passkeys {
            Some(passkeys) => app.app_data(passkeys.clone()),
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

macro_rules! json_of {
    ($app:expr, $req:expr) => {{
        match test::try_call_service(&$app, $req.to_request()).await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let bytes = test::read_body(resp).await;
                let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                (status, headers, body)
            }
            Err(err) => (
                err.as_response_error().status_code(),
                actix_web::http::header::HeaderMap::new(),
                Value::Null,
            ),
        }
    }};
}

struct Env {
    state: web::Data<AppState>,
    passkeys: Option<web::Data<Passkeys>>,
    raw: PgPool,
}

async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let redis = get_shared_redis().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let raw = get_raw_pool(host).await;
    // A new instance refuses passkeys: the administrator allows them.
    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = true")
        .execute(&raw)
        .await
        .unwrap();
    Env {
        state,
        passkeys: Some(web::Data::new(Passkeys::new(RP_ID, ORIGIN).unwrap())),
        raw,
    }
}

async fn create_account(raw: &PgPool) -> (i32, String) {
    let email = format!("passkey.{}@example.com", Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, phone_number, status, \
         first_connect, is_archived) \
         VALUES ('Pass', 'Key', $1, $2, '0612345678', 'active', false, false) RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (id, email)
}

async fn bearer(state: &AppState, user_id: u64) -> (&'static str, String) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&format!("pk_{}", Uuid::new_v4())),
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    let jwt = generate_session_jwt(user_id, session_id, "").unwrap();
    ("Authorization", format!("Bearer {jwt}"))
}

fn post(uri: &str, body: &Value) -> test::TestRequest {
    test::TestRequest::post().uri(uri).set_json(body)
}

/// The software authenticator cannot hold resident keys (it answers `NotSupported`), unlike the browsers
/// the options are meant for: the test lowers that one requirement, the rest is left as sent.
fn for_software_authenticator(options: &Value) -> CreationChallengeResponse {
    let mut options = options.clone();
    options["publicKey"]["authenticatorSelection"]["residentKey"] = json!("preferred");
    options["publicKey"]["authenticatorSelection"]["requireResidentKey"] = json!(false);
    serde_json::from_value(options).unwrap()
}

fn origin() -> Url {
    Url::parse(ORIGIN).unwrap()
}

/// The browser registers a passkey on `device`: options, authenticator, answer. Returns the
/// status and body of the final call.
macro_rules! register {
    ($app:expr, $device:expr, $auth:expr, $label:expr) => {{
        let (status, _, body) = json_of!(
            $app,
            post("/api/v1/user/me/passkeys/options/", &json!({})).insert_header($auth.clone())
        );
        assert_eq!(status, StatusCode::OK, "registration options: {body}");
        let options = for_software_authenticator(&body["options"]);
        let credential = $device.do_registration(origin(), options).unwrap();
        json_of!(
            $app,
            post(
                "/api/v1/user/me/passkeys/",
                &json!({ "label": $label, "credential": credential })
            )
            .insert_header($auth.clone())
        )
    }};
}

/// Credential id (base64url) of the passkey stored for `user_id`.
async fn stored_credential_id(raw: &PgPool, user_id: i32) -> String {
    let id: Vec<u8> = sqlx::query("SELECT credential_id FROM user_passkeys WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(raw)
        .await
        .unwrap()
        .get("credential_id");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id)
}

/// What a browser puts in `userHandle` for the account `user_id`.
fn handle_of(user_id: i32) -> String {
    let handle = core_api::passkeys::user_handle(u64::try_from(user_id).unwrap());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(handle.as_bytes())
}

/// Fetches a sign-in challenge and signs it on `device`; returns `(ceremony id, credential)`.
///
/// The software authenticator only signs for a listed credential and never returns the user handle
/// a real discoverable passkey carries, so the test does both, as the browser would: it names the
/// credential and sets `userHandle` to the one of `handle_user`.
macro_rules! sign_challenge {
    ($app:expr, $device:expr, $raw:expr, $user_id:expr, $handle_user:expr) => {{
        let (status, _, body) = json_of!($app, post("/api/v1/auth/passkey/options", &json!({})));
        assert_eq!(status, StatusCode::OK, "sign-in options: {body}");
        let mut options = body["options"].clone();
        options["publicKey"]["allowCredentials"] = json!([{
            "type": "public-key",
            "id": stored_credential_id(&$raw, $user_id).await
        }]);
        let options: RequestChallengeResponse = serde_json::from_value(options).unwrap();
        let credential = $device.do_authentication(origin(), options).unwrap();
        let mut credential = serde_json::to_value(credential).unwrap();
        credential["response"]["userHandle"] = json!(handle_of($handle_user));
        (body["id"].as_str().unwrap().to_string(), credential)
    }};
}

macro_rules! passkey_login {
    ($app:expr, $id:expr, $credential:expr) => {
        json_of!(
            $app,
            post(
                "/api/v1/auth/passkey",
                &json!({ "id": $id, "credential": $credential, "device_info": "test" })
            )
        )
    };
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_register_sign_in_and_delete_a_passkey() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);

    let (status, _, body) = register!(app, device, auth, "Town hall laptop");
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["label"], "Town hall laptop");
    assert_eq!(body["last_used_at"], Value::Null);
    let passkey_id = body["id"].as_i64().unwrap();
    assert!(
        body.get("passkey").is_none() && body.get("credential").is_none(),
        "the key is never returned"
    );
    let sent = fake_resend::sent_to(&email);
    assert_eq!(sent.len(), 1, "the agent is told about the new passkey");

    let (status, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["id"].as_i64(), Some(passkey_id));
    assert!(list[0].get("passkey").is_none());

    // Sign in with the passkey alone: a session, like the password login.
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, id);
    let (status, headers, body) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("Bearer ")));
    assert!(body["refresh_token"]
        .as_str()
        .is_some_and(|t| !t.is_empty()));
    let (_, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(auth.clone())
    );
    assert_ne!(list[0]["last_used_at"], Value::Null, "the use is recorded");

    // The answer of the first sign-in cannot be replayed: the challenge is burnt.
    let replay = passkey_login!(app, ceremony, credential);
    assert_eq!(replay.0, StatusCode::UNAUTHORIZED);

    // A sign-in prepared while the passkey still exists, submitted after its deletion.
    let (pending_ceremony, pending_credential) = sign_challenge!(app, device, env.raw, id, id);

    // Deleting it: 204, an e-mail, and the passkey no longer opens a session.
    let (status, _, _) = json_of!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/user/me/passkeys/{passkey_id}/"))
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(fake_resend::sent_to(&email).len(), 2);
    let (status, headers, _) = passkey_login!(app, pending_ceremony, pending_credential);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!headers.contains_key("Authorization"));
    let (_, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(auth.clone())
    );
    assert_eq!(list, json!([]));
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_a_passkey_of_another_account_cannot_be_deleted_and_stays_private() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (owner, _) = create_account(&env.raw).await;
    let (other, _) = create_account(&env.raw).await;
    let owner_auth = bearer(&env.state, u64::try_from(owner).unwrap()).await;
    let other_auth = bearer(&env.state, u64::try_from(other).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (_, _, body) = register!(app, device, owner_auth, "Mine");
    let passkey_id = body["id"].as_i64().unwrap();

    let (status, _, _) = json_of!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/user/me/passkeys/{passkey_id}/"))
            .insert_header(other_auth.clone())
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(other_auth)
    );
    assert_eq!(list, json!([]));
    let (_, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(owner_auth)
    );
    assert_eq!(list.as_array().unwrap().len(), 1, "still there");
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_an_archived_account_cannot_sign_in_with_its_passkey() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, _) = register!(app, device, auth, "Phone");
    assert_eq!(status, StatusCode::CREATED);
    sqlx::query("UPDATE users SET is_archived = true WHERE id = $1")
        .bind(id)
        .execute(&env.raw)
        .await
        .unwrap();
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, id);
    let (status, headers, _) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!headers.contains_key("Authorization"));
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_a_counter_that_went_back_is_refused_as_a_cloned_passkey() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, _) = register!(app, device, auth, "Phone");
    assert_eq!(status, StatusCode::CREATED);
    // The server already saw a much higher counter than what the (cloned) device sends.
    sqlx::query(
        "UPDATE user_passkeys SET passkey = jsonb_set(passkey, '{cred,counter}', '1000') \
         WHERE user_id = $1",
    )
    .bind(id)
    .execute(&env.raw)
    .await
    .unwrap();
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, id);
    let (status, headers, _) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!headers.contains_key("Authorization"));
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_a_registration_challenge_is_read_once_and_needs_a_start() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);

    // No registration started.
    let (_, _, body) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/options/", &json!({})).insert_header(auth.clone())
    );
    let options = for_software_authenticator(&body["options"]);
    let credential = device.do_registration(origin(), options).unwrap();
    let answer = json!({ "label": "Phone", "credential": credential });
    let (first, _, _) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/", &answer).insert_header(auth.clone())
    );
    assert_eq!(first, StatusCode::CREATED);
    let (replayed, _, _) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/", &answer).insert_header(auth.clone())
    );
    assert_eq!(replayed, StatusCode::BAD_REQUEST, "the challenge is burnt");

    // An empty label is a 400 before anything is verified.
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/user/me/passkeys/",
            &json!({ "label": "  ", "credential": answer["credential"] })
        )
        .insert_header(auth)
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_passkeys_answer_503_without_webauthn_configuration() {
    let env = env().await;
    let none: Option<web::Data<Passkeys>> = None;
    let app = init_app!(env.state, none);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;

    let (status, _, _) = json_of!(app, post("/api/v1/auth/passkey/options", &json!({})));
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (status, _, _) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/options/", &json!({})).insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    // And the administrator cannot allow passkeys nothing can serve.
    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (status, _, _) = json_of!(
        app,
        test::TestRequest::put()
            .uri("/api/v1/admin/two-factor-policy")
            .set_json(json!({
                "required": false, "totp_allowed": true,
                "passkey_allowed": true, "backup_codes_allowed": true
            }))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_the_policy_gates_passkeys_and_the_administrator_may_allow_them() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;

    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = false")
        .execute(&env.raw)
        .await
        .unwrap();
    let (status, _, _) = json_of!(app, post("/api/v1/auth/passkey/options", &json!({})));
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/options/", &json!({})).insert_header(auth)
    );
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, body) = json_of!(
        app,
        test::TestRequest::put()
            .uri("/api/v1/admin/two-factor-policy")
            .set_json(json!({
                "required": false, "totp_allowed": true,
                "passkey_allowed": true, "backup_codes_allowed": true
            }))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["passkey_allowed"], json!(true));
    let (status, _, _) = json_of!(app, post("/api/v1/auth/passkey/options", &json!({})));
    assert_eq!(status, StatusCode::OK);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_unknown_or_forged_answers_get_the_same_401() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, _) = register!(app, device, auth, "Phone");
    assert_eq!(status, StatusCode::CREATED);

    // An unknown ceremony id.
    let (_, credential) = sign_challenge!(app, device, env.raw, id, id);
    let (status, _, _) = passkey_login!(app, "does-not-exist", credential);
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A passkey nobody registered, on a real challenge.
    let mut stranger = SoftPasskey::new(true);
    let (_, _, body) = json_of!(app, post("/api/v1/auth/passkey/options", &json!({})));
    let options: RequestChallengeResponse =
        serde_json::from_value(body["options"].clone()).unwrap();
    // The stranger holds no credential for this site: the authenticator itself refuses, so the
    // browser never sends an answer. A body without credential is a 400.
    assert!(stranger.do_authentication(origin(), options).is_err());
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey",
            &json!({ "id": body["id"], "device_info": "test" })
        )
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_a_passkey_presented_with_another_accounts_handle_is_refused() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let (other, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, _) = register!(app, device, auth, "Phone");
    assert_eq!(status, StatusCode::CREATED);
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, other);
    let (status, headers, _) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!headers.contains_key("Authorization"));
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_sign_in_is_refused_once_the_policy_stops_allowing_passkeys() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, _) = register!(app, device, auth, "Phone");
    assert_eq!(status, StatusCode::CREATED);
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, id);
    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = false")
        .execute(&env.raw)
        .await
        .unwrap();
    let (status, headers, _) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(!headers.contains_key("Authorization"));
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_a_passkey_registered_on_one_front_signs_in_on_another() {
    let env = env().await;
    // Registered from the settings front, used from the login front: both origins of one `rpId`.
    let passkeys = Some(web::Data::new(
        Passkeys::new(
            "localhost",
            "https://login.localhost, https://settings.localhost",
        )
        .unwrap(),
    ));
    let app = init_app!(env.state, passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);

    let (_, _, body) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/options/", &json!({})).insert_header(auth.clone())
    );
    let options = for_software_authenticator(&body["options"]);
    let credential = device
        .do_registration(Url::parse("https://settings.localhost").unwrap(), options)
        .unwrap();
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/user/me/passkeys/",
            &json!({ "label": "Phone", "credential": credential })
        )
        .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::CREATED);

    let (_, _, body) = json_of!(app, post("/api/v1/auth/passkey/options", &json!({})));
    let mut options = body["options"].clone();
    options["publicKey"]["allowCredentials"] = json!([{
        "type": "public-key",
        "id": stored_credential_id(&env.raw, id).await
    }]);
    let options: RequestChallengeResponse = serde_json::from_value(options).unwrap();
    let credential = device
        .do_authentication(Url::parse("https://login.localhost").unwrap(), options)
        .unwrap();
    let mut credential = serde_json::to_value(credential).unwrap();
    credential["response"]["userHandle"] = json!(handle_of(id));
    let (status, headers, _) = passkey_login!(app, body["id"].as_str().unwrap(), credential);
    assert_eq!(status, StatusCode::OK);
    assert!(headers.contains_key("Authorization"));
}

/// A token of the mandatory enrolment, as `POST /api/v1/auth/login` hands it out.
async fn setup_token(env: &Env, user_id: i32) -> String {
    core_api::two_factor::begin_setup(
        env.state.get_redis(),
        u64::try_from(user_id).unwrap(),
        "enrolment device",
    )
    .await
    .unwrap()
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_the_enrolment_token_registers_a_passkey_and_opens_the_session() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, email) = create_account(&env.raw).await;
    let token = setup_token(&env, id).await;
    let mut device = SoftPasskey::new(true);

    let (status, _, body) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    let options = for_software_authenticator(&body["options"]);
    let credential = device.do_registration(origin(), options).unwrap();
    let (status, headers, body) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup",
            &json!({ "token": token, "label": "Town hall laptop", "credential": credential })
        )
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("Bearer ")));
    assert!(body["refresh_token"]
        .as_str()
        .is_some_and(|t| !t.is_empty()));
    assert!(
        body.get("backup_codes").is_none(),
        "a passkey is a complete factor: no backup codes"
    );
    assert_eq!(fake_resend::sent_to(&email).len(), 1, "the agent is told");

    // The passkey is stored under its label and signs the agent in afterwards.
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, id);
    let (status, _, _) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::OK);

    // The token is spent: neither the options nor a second answer work with it.
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_the_enrolment_by_passkey_refuses_without_leaking() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;

    // Unknown token on both routes: the same 401, no ceremony started.
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": "never-issued" })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let mut device = SoftPasskey::new(true);
    let token = setup_token(&env, id).await;
    let (_, _, body) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    let options = for_software_authenticator(&body["options"]);
    let credential = device.do_registration(origin(), options).unwrap();
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup",
            &json!({ "token": "never-issued", "label": "x", "credential": credential })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // An answer without a started ceremony is a 400 and the token is burnt after five of them.
    // (The ceremony is kept per account: another account has none pending.)
    let (no_ceremony, _) = create_account(&env.raw).await;
    let other = setup_token(&env, no_ceremony).await;
    for attempt in 1..=5 {
        let (status, _, _) = json_of!(
            app,
            post(
                "/api/v1/auth/passkey/setup",
                &json!({ "token": other, "label": "x", "credential": credential })
            )
        );
        let expected = if attempt < 5 {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::UNAUTHORIZED
        };
        assert_eq!(status, expected, "attempt {attempt}");
    }
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": other })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED, "the token is burnt");

    // Nothing was stored and no session was opened.
    let stored: i64 = sqlx::query("SELECT count(*) AS n FROM user_passkeys WHERE user_id = $1")
        .bind(no_ceremony)
        .fetch_one(&env.raw)
        .await
        .unwrap()
        .get("n");
    assert_eq!(stored, 0);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_the_enrolment_by_passkey_follows_the_policy_and_the_configuration() {
    let env = env().await;
    let (id, _) = create_account(&env.raw).await;

    // The administrator no longer accepts passkeys: 403 on both routes.
    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = false")
        .execute(&env.raw)
        .await
        .unwrap();
    let app = init_app!(env.state, env.passkeys);
    let token = setup_token(&env, id).await;
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    assert_eq!(status, StatusCode::FORBIDDEN);
    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = true")
        .execute(&env.raw)
        .await
        .unwrap();

    // No relying party on the instance: 503, so the front falls back to another mode.
    let unconfigured: Option<web::Data<Passkeys>> = None;
    let app = init_app!(env.state, unconfigured);
    let token = setup_token(&env, id).await;
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

    // Malformed bodies never reach the ceremony.
    let (status, _, _) = json_of!(
        app,
        post("/api/v1/auth/passkey/setup", &json!({ "token": token }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

macro_rules! password_login {
    ($app:expr, $email:expr) => {
        json_of!(
            $app,
            post(
                "/api/v1/auth/login",
                &json!({ "email": $email, "password": PASSWORD, "device_info": "test" })
            )
        )
    };
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_the_password_login_tells_whether_to_propose_a_passkey() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, email) = create_account(&env.raw).await;

    // Passkeys accepted, none registered yet: the screen proposes one.
    let (status, _, body) = password_login!(app, email);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["passkey_offer"],
        json!({ "allowed": true, "registered": false })
    );

    // Once the agent has one, nothing to propose.
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, _) = register!(app, device, auth, "Laptop");
    assert_eq!(status, StatusCode::CREATED);
    let (_, _, body) = password_login!(app, email);
    assert_eq!(
        body["passkey_offer"],
        json!({ "allowed": true, "registered": true })
    );

    // A sign-in by passkey is not asked to propose one.
    let (ceremony, credential) = sign_challenge!(app, device, env.raw, id, id);
    let (status, _, body) = passkey_login!(app, ceremony, credential);
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("passkey_offer").is_none(), "{body}");

    // The administrator stops accepting passkeys: not allowed.
    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = false")
        .execute(&env.raw)
        .await
        .unwrap();
    let (_, _, body) = password_login!(app, email);
    assert_eq!(body["passkey_offer"]["allowed"], json!(false));
    sqlx::query("UPDATE two_factor_policy SET passkey_allowed = true")
        .execute(&env.raw)
        .await
        .unwrap();

    // No relying party on the instance: not allowed either.
    let unconfigured: Option<web::Data<Passkeys>> = None;
    let app = init_app!(env.state, unconfigured);
    let (_, _, body) = password_login!(app, email);
    assert_eq!(body["passkey_offer"]["allowed"], json!(false));
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_the_passkey_manager_is_kept_and_listed_when_known() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;

    // The software authenticator gives no known AAGUID: the passkey has no manager name.
    let mut device = SoftPasskey::new(true);
    let (status, _, body) = register!(app, device, auth, "Laptop");
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["provider"], Value::Null);

    // A name deduced at registration is stored and listed (the AAGUID lookup itself is unit-tested).
    sqlx::query(
        "INSERT INTO user_passkeys (user_id, credential_id, passkey, label, provider) \
         VALUES ($1, '\\x0a0b'::bytea, '{}'::jsonb, 'Phone', 'Google Password Manager')",
    )
    .bind(id)
    .execute(&env.raw)
    .await
    .unwrap();
    let (status, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::OK);
    let providers: Vec<&Value> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| &p["provider"])
        .collect();
    assert_eq!(providers, [&Value::Null, &json!("Google Password Manager")]);
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_registration_options_require_a_discoverable_credential_on_both_routes() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;

    // The sign-in names no account: a passkey that is not discoverable could never be offered.
    let (_, _, signed_in) = json_of!(
        app,
        post("/api/v1/user/me/passkeys/options/", &json!({})).insert_header(auth.clone())
    );
    let token = setup_token(&env, id).await;
    let (_, _, enrolment) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    for options in [signed_in, enrolment] {
        let selection = &options["options"]["publicKey"]["authenticatorSelection"];
        assert_eq!(selection["residentKey"], json!("required"), "{options}");
        assert_eq!(selection["requireResidentKey"], json!(true), "{options}");
        assert_eq!(
            selection["userVerification"],
            json!("required"),
            "{options}"
        );
        assert!(
            selection.get("authenticatorAttachment").is_none(),
            "any password manager"
        );
    }
}

#[actix_web::test]
#[serial(fake_resend)]
async fn mair_810_a_token_of_an_archived_or_already_protected_account_enrols_nothing() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);

    // Archived after the token was issued: refused like an unknown token, nothing stored.
    let (archived, _) = create_account(&env.raw).await;
    let token = setup_token(&env, archived).await;
    sqlx::query("UPDATE users SET is_archived = true WHERE id = $1")
        .bind(archived)
        .execute(&env.raw)
        .await
        .unwrap();
    let (status, _, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Protected by another enrolment after the token was issued: no passkey, no session.
    let (protected, _) = create_account(&env.raw).await;
    let token = setup_token(&env, protected).await;
    sqlx::query(
        "INSERT INTO user_totp (user_id, secret_encrypted, confirmed_at) VALUES ($1, '\\xdeadbeef', now())",
    )
    .bind(protected)
    .execute(&env.raw)
    .await
    .unwrap();
    let (status, headers, _) = json_of!(
        app,
        post(
            "/api/v1/auth/passkey/setup/options",
            &json!({ "token": token })
        )
    );
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(!headers.contains_key("Authorization"));
    let stored: i64 = sqlx::query_scalar("SELECT count(*) FROM user_passkeys WHERE user_id = $1")
        .bind(protected)
        .fetch_one(&env.raw)
        .await
        .unwrap();
    assert_eq!(stored, 0);
}

// MAIR-811: the Login screen registers the passkey, then saves the name the agent keeps (the passkey
// manager by default).
#[actix_web::test]
#[serial(fake_resend)]
async fn mair_811_rename_a_passkey() {
    let env = env().await;
    let app = init_app!(env.state, env.passkeys);
    let (owner, email) = create_account(&env.raw).await;
    let (other, _) = create_account(&env.raw).await;
    let owner_auth = bearer(&env.state, u64::try_from(owner).unwrap()).await;
    let other_auth = bearer(&env.state, u64::try_from(other).unwrap()).await;
    let mut device = SoftPasskey::new(true);
    let (status, _, created) = register!(app, device, owner_auth, "Passkey");
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let passkey_id = created["id"].as_i64().unwrap();
    let mails = fake_resend::sent_to(&email).len();

    let rename = |auth: (&'static str, String), id: String, body: Value| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/user/me/passkeys/{id}/"))
            .insert_header(auth)
            .set_json(body)
    };

    // The owner renames it: the new label (trimmed), the rest unchanged, no e-mail.
    let (status, _, body) = json_of!(
        app,
        rename(
            owner_auth.clone(),
            passkey_id.to_string(),
            json!({ "label": "  Ordinateur de la mairie  " })
        )
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"].as_i64(), Some(passkey_id));
    assert_eq!(body["label"], "Ordinateur de la mairie");
    assert_eq!(body["created_at"], created["created_at"]);
    assert_eq!(body["provider"], created["provider"]);
    assert!(body.get("passkey").is_none(), "the key is never returned");
    assert_eq!(
        fake_resend::sent_to(&email).len(),
        mails,
        "renaming sends no e-mail"
    );
    let (_, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(owner_auth.clone())
    );
    assert_eq!(list[0]["label"], "Ordinateur de la mairie");

    // Another account gets the same 404 as for an unknown passkey, and changes nothing.
    let (status, _, _) = json_of!(
        app,
        rename(
            other_auth,
            passkey_id.to_string(),
            json!({ "label": "Volée" })
        )
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = json_of!(
        app,
        rename(
            owner_auth.clone(),
            "999999999".to_string(),
            json!({ "label": "Inconnue" })
        )
    );
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A blank, too long or control-character label and a non numeric id are 400.
    for label in [json!("   "), json!("x".repeat(101)), json!("a\u{0}b")] {
        let (status, _, _) = json_of!(
            app,
            rename(
                owner_auth.clone(),
                passkey_id.to_string(),
                json!({ "label": label })
            )
        );
        assert_eq!(status, StatusCode::BAD_REQUEST, "{label}");
    }
    let (status, _, _) = json_of!(
        app,
        rename(
            owner_auth.clone(),
            "abc".to_string(),
            json!({ "label": "Ordinateur" })
        )
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, _, list) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/passkeys/")
            .insert_header(owner_auth)
    );
    assert_eq!(
        list[0]["label"], "Ordinateur de la mairie",
        "the refused renames changed nothing"
    );
}
