//! MAIR-805 / MAIR-808: the second factor (TOTP and backup codes) managed by Core, end to end on the real
//! routes: enrolment, two-step login, replay and burn-out protections, disabling, administrator
//! reset, and the password reset that must not bypass it.

use crate::common::core_db_url;
use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use base64::{engine::general_purpose, Engine as _};
use core_api::database::second_factor::{
    ConfirmTotpQueryView, DeleteTotpQueryView, LockActiveTotpQueryView, UseTotpStepQueryView,
    Written,
};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::second_factor::{code_for_step, now_unix, step_at, SecretCipher};
use core_api::session_jwt::generate_session_jwt;
use core_api::two_factor::{self, PendingLogin};
use data_encoding::BASE32_NOPAD;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::jwt_manager::is_session_revoked;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::get_shared_redis;
use mairie360_api_lib::{password::hash_password, security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";

/// Same wiring as `main.rs`, with the cipher of the second factor registered.
macro_rules! init_app {
    ($state:expr, $cipher:expr) => {
        test::init_service(
            App::new()
                .app_data($state.clone())
                .app_data($cipher.clone())
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

struct Env {
    state: web::Data<AppState>,
    cipher: web::Data<SecretCipher>,
    raw: PgPool,
}

async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let redis = get_shared_redis().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let cipher = web::Data::new(
        SecretCipher::from_base64(&general_purpose::STANDARD.encode([9u8; 32])).unwrap(),
    );
    Env {
        state,
        cipher,
        raw: get_raw_pool(host).await,
    }
}

/// Inserts an account with `PASSWORD` and returns `(id, email)`.
async fn create_account(raw: &PgPool) -> (i32, String) {
    let email = format!("twofactor.{}@example.com", Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, phone_number, status, \
         first_connect, is_archived) \
         VALUES ('Second', 'Factor', $1, $2, '0612345678', 'active', false, false) RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (id, email)
}

/// Opens a session for `user_id` as the login does and returns its `Authorization` header.
async fn bearer(state: &AppState, user_id: u64) -> (&'static str, String) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&format!("2fa_{}", Uuid::new_v4())),
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

fn totp_now(secret_base32: &str) -> String {
    let secret = BASE32_NOPAD.decode(secret_base32.as_bytes()).unwrap();
    code_for_step(&secret, step_at(now_unix()))
}

fn totp_at(secret_base32: &str, step: u64) -> String {
    let secret = BASE32_NOPAD.decode(secret_base32.as_bytes()).unwrap();
    code_for_step(&secret, step)
}

macro_rules! json_of {
    ($app:expr, $req:expr) => {{
        match test::try_call_service(&$app, $req.to_request()).await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let bytes = test::read_body(resp).await;
                let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                (
                    status,
                    headers,
                    body,
                    String::from_utf8_lossy(&bytes).to_string(),
                )
            }
            // A guard (`JwtMiddleware`, `session_guard`) refuses with an error, not a response.
            Err(err) => (
                err.as_response_error().status_code(),
                actix_web::http::header::HeaderMap::new(),
                Value::Null,
                err.to_string(),
            ),
        }
    }};
}

/// Enrols the account (setup + confirm) and returns `(base32 secret, backup codes)`.
macro_rules! enrol {
    ($app:expr, $auth:expr) => {{
        let (secret, codes, _) = enrol_full!($app, $auth);
        (secret, codes)
    }};
}

/// Same as `enrol!`, plus the code that confirmed the enrolment (the one the step spends).
macro_rules! enrol_full {
    ($app:expr, $auth:expr) => {{
        let (status, _, body, _) = json_of!(
            $app,
            test::TestRequest::post()
                .uri("/api/v1/user/me/2fa/setup/")
                .insert_header($auth.clone())
        );
        assert_eq!(status, StatusCode::OK);
        let secret = body["secret"].as_str().unwrap().to_string();
        let confirm_code = totp_now(&secret);
        let (status, _, body, _) = json_of!(
            $app,
            post("/api/v1/user/me/2fa/confirm/", &json!({ "code": confirm_code }))
                .insert_header($auth.clone())
        );
        assert_eq!(status, StatusCode::OK);
        let codes: Vec<String> = body["backup_codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap().to_string())
            .collect();
        (secret, codes, confirm_code)
    }};
}

/// Password login; returns the response parts.
macro_rules! login {
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
async fn enrolment_gives_ten_backup_codes_and_turns_the_factor_on() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;

    let (status, _, body, _) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/2fa/")
            .insert_header(auth.clone())
    );
    assert_eq!(
        (status, body["enabled"].clone()),
        (StatusCode::OK, json!(false))
    );

    let (status, _, setup, _) = json_of!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/user/me/2fa/setup/")
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::OK);
    let secret = setup["secret"].as_str().unwrap();
    assert!(setup["otpauth_uri"].as_str().unwrap().contains(secret));

    // Pending: not active yet, a wrong code keeps it pending.
    let (status, ..) = json_of!(
        app,
        post("/api/v1/user/me/2fa/confirm/", &json!({ "code": "000000" }))
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, _, body, _) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/2fa/")
            .insert_header(auth.clone())
    );
    assert_eq!(body["enabled"], json!(false));

    let (status, _, body, _) = json_of!(
        app,
        post(
            "/api/v1/user/me/2fa/confirm/",
            &json!({ "code": totp_now(secret) })
        )
        .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["backup_codes"].as_array().unwrap().len(), 10);

    let (_, _, body, _) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/2fa/")
            .insert_header(auth.clone())
    );
    assert_eq!(
        body,
        json!({ "enabled": true, "backup_codes_remaining": 10 })
    );

    // Active: a new enrolment is refused until it is disabled.
    let (status, ..) = json_of!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/user/me/2fa/setup/")
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::CONFLICT);

    // The secret is sealed in the database, never in clear.
    let sealed: Vec<u8> = sqlx::query("SELECT secret_encrypted FROM user_totp WHERE user_id = $1")
        .bind(id)
        .fetch_one(&env.raw)
        .await
        .unwrap()
        .get("secret_encrypted");
    let clear = BASE32_NOPAD.decode(secret.as_bytes()).unwrap();
    assert!(!sealed.windows(clear.len()).any(|w| w == clear.as_slice()));
}

#[actix_web::test]
async fn a_protected_account_gets_no_session_from_its_password() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (secret, _, spent_code) = enrol_full!(app, auth);

    let (status, headers, body, _) = login!(app, email);
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(
        headers.get("Authorization").is_none(),
        "no JWT before the second step"
    );
    assert_eq!(body["status"], json!("2fa_required"));
    let token = body["token"].as_str().unwrap().to_string();

    // The login token opens no route.
    let (status, ..) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/")
            .insert_header(("Authorization", format!("Bearer {token}")))
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A wrong code keeps the token; the right one opens the session.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": "000000" })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // The code of the step used to confirm the enrolment is spent: it cannot log in.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": spent_code })
        )
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the code that confirmed the enrolment is spent"
    );
    // The next step's code is within the tolerated drift and later than the spent one.
    let next = totp_at(&secret, step_at(now_unix()) + 1);
    let (status, headers, body, _) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": next })
        )
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(headers
        .get("Authorization")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("Bearer "));
    assert!(body["refresh_token"].as_str().is_some());

    // The token is single use, and so is the code.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": next })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": next })
        )
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a TOTP code cannot be replayed"
    );
}

#[actix_web::test]
async fn a_backup_code_logs_in_once() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (_, codes) = enrol!(app, auth);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[0].to_uppercase() })
        )
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "typed in upper case, the code is still accepted"
    );

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, _, _, text) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[0] })
        )
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a spent backup code is refused"
    );
    assert_eq!(text, "Backup code already used.");
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[1] })
        )
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "another backup code still works on the same token"
    );

    let (_, _, status_body, _) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/2fa/")
            .insert_header(bearer(&env.state, u64::try_from(id).unwrap()).await)
    );
    assert_eq!(status_body["backup_codes_remaining"], json!(8));
}

#[actix_web::test]
async fn the_login_token_is_burnt_after_five_wrong_codes() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (secret, _) = enrol!(app, auth);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    for _ in 0..5 {
        let (status, ..) = json_of!(
            app,
            post(
                "/api/v1/auth/login/2fa",
                &json!({ "token": token, "code": "000000" })
            )
        );
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // Even the right code is refused now: the agent starts over with the password.
    let next = totp_at(&secret, step_at(now_unix()) + 1);
    let (status, _, _, text) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": next })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(text, "Invalid or expired login token.");
}

#[actix_web::test]
async fn disabling_needs_the_password_and_a_code() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (secret, codes) = enrol!(app, auth);

    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/user/me/2fa/disable/",
            &json!({ "password": "Wrong!Pass123", "code": codes[0] })
        )
        .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/user/me/2fa/disable/",
            &json!({ "password": PASSWORD, "code": "000000" })
        )
        .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, _, body, _) = login!(app, email);
    assert_eq!(
        body["status"],
        json!("2fa_required"),
        "still protected after the refusals"
    );

    let _ = secret;
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/user/me/2fa/disable/",
            &json!({ "password": PASSWORD, "code": codes[2] })
        )
        .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, headers, ..) = login!(app, email);
    assert_eq!(status, StatusCode::OK, "password only again");
    assert!(headers.get("Authorization").is_some());
    let remaining: i64 =
        sqlx::query("SELECT count(*) AS n FROM user_backup_codes WHERE user_id = $1")
            .bind(id)
            .fetch_one(&env.raw)
            .await
            .unwrap()
            .get("n");
    assert_eq!(remaining, 0);
}

#[actix_web::test]
async fn regenerating_the_backup_codes_invalidates_the_old_ones() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (_, old) = enrol!(app, auth);

    let (status, _, body, _) = json_of!(
        app,
        post(
            "/api/v1/user/me/2fa/backup-codes/",
            &json!({ "code": old[0] })
        )
        .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::OK);
    let new = body["backup_codes"].as_array().unwrap();
    assert_eq!(new.len(), 10);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": old[1] })
        )
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the former codes stopped working"
    );
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": new[0] })
        )
    );
    assert_eq!(status, StatusCode::OK);
}

#[actix_web::test]
async fn an_administrator_reset_removes_the_factor_and_closes_the_sessions() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let uid = u64::try_from(id).unwrap();
    let auth = bearer(&env.state, uid).await;
    enrol!(app, auth);

    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (status, ..) = json_of!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{id}/2fa"))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The session of the agent is closed, the factor gone.
    let (status, ..) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/2fa/")
            .insert_header(auth)
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, headers, ..) = login!(app, email);
    assert_eq!(status, StatusCode::OK);
    assert!(headers.get("Authorization").is_some());

    // Idempotent for a user without a factor, 404 for an unknown one, 403 for a non-admin.
    let (status, ..) = json_of!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{id}/2fa"))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, ..) = json_of!(
        app,
        test::TestRequest::delete()
            .uri("/api/v1/admin/users/99999999/2fa")
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
    let plain = bearer(&env.state, uid).await;
    let (status, ..) = json_of!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{id}/2fa"))
            .insert_header(plain)
    );
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn a_login_token_does_not_survive_the_reset_of_the_factor() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (secret, _) = enrol!(app, auth);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    sqlx::query("DELETE FROM user_totp WHERE user_id = $1")
        .bind(id)
        .execute(&env.raw)
        .await
        .unwrap();
    let next = totp_at(&secret, step_at(now_unix()) + 1);
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": next })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn the_second_factor_is_off_without_the_encryption_key() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let redis = get_shared_redis().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let raw = get_raw_pool(host).await;
    let (id, email) = create_account(&raw).await;
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(public_config)
            .service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
    )
    .await;
    let auth = bearer(&state, u64::try_from(id).unwrap()).await;
    let (status, ..) = json_of!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/user/me/2fa/setup/")
            .insert_header(auth)
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    // An account without a factor still logs in.
    let (status, ..) = login!(app, email);
    assert_eq!(status, StatusCode::OK);
    // One that has a factor is refused (503) rather than opened without it.
    sqlx::query("INSERT INTO user_totp (user_id, secret_encrypted, confirmed_at) VALUES ($1, '\\xdeadbeef', now())")
        .bind(id)
        .execute(&raw)
        .await
        .unwrap();
    let (status, headers, ..) = login!(app, email);
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(headers.get("Authorization").is_none());
}

#[actix_web::test]
async fn a_password_reset_does_not_bypass_the_second_factor() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (secret, _) = enrol!(app, auth);

    // The token the reset e-mail carries, as `forgot_password` stores it.
    let reset_token = Uuid::new_v4().to_string();
    env.state
        .get_redis()
        .secure_set_ex(
            &format!("{reset_token}/forgot_password_email"),
            email.clone(),
            900,
        )
        .await
        .unwrap();
    let (status, headers, body, _) = json_of!(
        app,
        post(
            "/api/v1/auth/reset_password",
            &json!({ "token": reset_token, "new_password": "Brand!New123", "device_info": "test" })
        )
    );
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "owning the mailbox is not a second factor"
    );
    assert!(headers.get("Authorization").is_none());
    assert_eq!(body["status"], json!("2fa_required"));

    // The password was reset; the second step opens the session.
    let next = totp_at(&secret, step_at(now_unix()) + 1);
    let (status, headers, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": body["token"], "code": next })
        )
    );
    assert_eq!(status, StatusCode::OK);
    assert!(headers.get("Authorization").is_some());
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login",
            &json!({ "email": email, "password": "Brand!New123", "device_info": "test" })
        )
    );
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "the new password is the one that counts"
    );
}

// ---------------------------------------------------------------------------------------------
// MAIR-808: instance policy (mandatory second factor, allowed modes)
// ---------------------------------------------------------------------------------------------

fn put_policy(
    required: bool,
    totp: bool,
    passkey: bool,
    backup: bool,
    auth: &(&'static str, String),
) -> test::TestRequest {
    test::TestRequest::put()
        .uri("/api/v1/admin/two-factor-policy")
        .set_json(json!({
            "required": required,
            "totp_allowed": totp,
            "passkey_allowed": passkey,
            "backup_codes_allowed": backup
        }))
        .insert_header(auth.clone())
}

#[actix_web::test]
async fn mair_808_the_policy_is_read_and_written_by_administrators_only_and_audited() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin_id = u64::try_from(*ADMIN_ID.get().unwrap()).unwrap();
    let admin = bearer(&env.state, admin_id).await;
    let (uid, _) = create_account(&env.raw).await;
    let plain = bearer(&env.state, u64::try_from(uid).unwrap()).await;

    let (status, _, body, _) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/admin/two-factor-policy")
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (
            &body["required"],
            &body["totp_allowed"],
            &body["passkey_allowed"],
            &body["backup_codes_allowed"],
            &body["updated_by"]
        ),
        (
            &json!(false),
            &json!(true),
            &json!(false),
            &json!(true),
            &Value::Null
        ),
        "a new instance: optional, TOTP and backup codes allowed"
    );

    // An agent can neither read nor write it.
    let (status, ..) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/admin/two-factor-policy")
            .insert_header(plain.clone())
    );
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, ..) = json_of!(app, put_policy(true, true, false, true, &plain));
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, ..) = json_of!(
        app,
        test::TestRequest::get().uri("/api/v1/admin/two-factor-policy")
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Invalid policies are refused: no mode, passkeys not available, unknown mode.
    let (status, ..) = json_of!(app, put_policy(true, false, false, true, &admin));
    assert_eq!(status, StatusCode::BAD_REQUEST, "no mode left");
    let (status, ..) = json_of!(app, put_policy(true, true, true, true, &admin));
    assert_eq!(status, StatusCode::BAD_REQUEST, "passkeys do not exist yet");
    let (status, ..) = json_of!(
        app,
        test::TestRequest::put()
            .uri("/api/v1/admin/two-factor-policy")
            .set_json(json!({
                "required": true, "totp_allowed": true, "passkey_allowed": false,
                "backup_codes_allowed": true, "sms_allowed": true
            }))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "SMS is not a mode");
    let (status, ..) = json_of!(
        app,
        test::TestRequest::put()
            .uri("/api/v1/admin/two-factor-policy")
            .set_json(json!({ "required": true }))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "the whole policy is sent");
    let audited: i64 = sqlx::query("SELECT count(*) FROM two_factor_policy_audit_log")
        .fetch_one(&env.raw)
        .await
        .unwrap()
        .get(0);
    assert_eq!(audited, 0, "a refused policy leaves no trace");

    let (status, _, body, _) = json_of!(app, put_policy(true, true, false, false, &admin));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["required"], json!(true));
    assert_eq!(body["backup_codes_allowed"], json!(false));
    assert_eq!(body["updated_by"], json!(admin_id));
    let (_, _, again, _) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/admin/two-factor-policy")
            .insert_header(admin.clone())
    );
    assert_eq!(again, body, "the write is what the read returns");

    // The change is in the audit log: author, before, after.
    let row = sqlx::query(
        "SELECT action_by, (previous_data ->> 'required')::bool AS before, \
         (new_data ->> 'required')::bool AS after FROM two_factor_policy_audit_log",
    )
    .fetch_one(&env.raw)
    .await
    .unwrap();
    assert_eq!(
        (
            row.get::<Option<i32>, _>("action_by"),
            row.get::<bool, _>("before"),
            row.get::<bool, _>("after")
        ),
        (Some(*ADMIN_ID.get().unwrap()), false, true)
    );
}

#[actix_web::test]
async fn mair_808_a_mandatory_policy_makes_an_agent_without_factor_enrol_before_any_session() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (id, email) = create_account(&env.raw).await;
    let (other_id, other_email) = create_account(&env.raw).await;
    // This agent already has a factor: the policy changes nothing for them but the 2nd step.
    let other_auth = bearer(&env.state, u64::try_from(other_id).unwrap()).await;
    enrol!(app, other_auth);

    let (status, ..) = json_of!(app, put_policy(true, true, false, true, &admin));
    assert_eq!(status, StatusCode::OK);

    // Right password, no session: only the enrolment state and its token.
    let (status, headers, body, _) = login!(app, email);
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], json!("2fa_setup_required"));
    assert!(
        headers.get("Authorization").is_none(),
        "no session before the enrolment"
    );
    let token = body["token"].as_str().unwrap().to_string();

    // The enrolment token opens no route: it is not a JWT.
    let (status, ..) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/")
            .insert_header(("Authorization", format!("Bearer {token}")))
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // Nor does it finish a login, nor can the login token of someone else enrol.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": "123456" })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/2fa/setup",
            &json!({ "token": "unknown-token" })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, _, body, _) = login!(app, other_email);
    assert_eq!(body["status"], json!("2fa_required"));
    let login_token = body["token"].as_str().unwrap();
    let (status, ..) = json_of!(
        app,
        post("/api/v1/auth/2fa/setup", &json!({ "token": login_token }))
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a login token cannot enrol"
    );

    // Confirm before setup: nothing in progress.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/2fa/confirm",
            &json!({ "token": token, "code": "123456" })
        )
    );
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, setup, _) = json_of!(
        app,
        post("/api/v1/auth/2fa/setup", &json!({ "token": token }))
    );
    assert_eq!(status, StatusCode::OK);
    let secret = setup["secret"].as_str().unwrap().to_string();

    let (status, headers, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/2fa/confirm",
            &json!({ "token": token, "code": "000000" })
        )
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a wrong code enrols nothing"
    );
    assert!(headers.get("Authorization").is_none());

    let (status, headers, body, _) = json_of!(
        app,
        post(
            "/api/v1/auth/2fa/confirm",
            &json!({ "token": token, "code": totp_now(&secret) })
        )
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["backup_codes"].as_array().unwrap().len(), 10);
    assert!(body["refresh_token"].as_str().is_some());
    let jwt = headers
        .get("Authorization")
        .expect("the session opens once the factor is enrolled")
        .to_str()
        .unwrap()
        .to_string();
    let (status, ..) = json_of!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/user/me/")
            .insert_header(("Authorization", jwt.clone()))
    );
    assert_eq!(status, StatusCode::OK, "the modules are reachable");

    // The token is spent; the next login asks for the code like any protected account.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/2fa/confirm",
            &json!({ "token": token, "code": totp_now(&secret) })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, body, _) = login!(app, email);
    assert_eq!(
        (status, &body["status"]),
        (StatusCode::ACCEPTED, &json!("2fa_required"))
    );

    // A mandatory policy forbids turning the factor off.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/user/me/2fa/disable/",
            &json!({ "password": PASSWORD, "code": totp_now(&secret) })
        )
        .insert_header(("Authorization", jwt))
    );
    assert_eq!(status, StatusCode::FORBIDDEN);
    let _ = id;
}

#[actix_web::test]
async fn mair_808_the_enrolment_token_is_burnt_after_five_wrong_codes() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (_, email) = create_account(&env.raw).await;
    let (status, ..) = json_of!(app, put_policy(true, true, false, true, &admin));
    assert_eq!(status, StatusCode::OK);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, ..) = json_of!(
        app,
        post("/api/v1/auth/2fa/setup", &json!({ "token": token }))
    );
    assert_eq!(status, StatusCode::OK);
    for _ in 0..4 {
        let (status, ..) = json_of!(
            app,
            post(
                "/api/v1/auth/2fa/confirm",
                &json!({ "token": token, "code": "000000" })
            )
        );
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, _, _, text) = json_of!(
        app,
        post(
            "/api/v1/auth/2fa/confirm",
            &json!({ "token": token, "code": "000000" })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        text.contains("enrolment token"),
        "the fifth failure burns it: {text}"
    );
    let (status, ..) = json_of!(
        app,
        post("/api/v1/auth/2fa/setup", &json!({ "token": token }))
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn mair_808_a_mode_removed_by_the_administrator_is_refused_at_login() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (secret, codes) = enrol!(app, auth);

    // Backup codes no longer accepted: the TOTP code still is.
    let (status, ..) = json_of!(app, put_policy(false, true, false, false, &admin));
    assert_eq!(status, StatusCode::OK);
    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[0] })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED, "backup codes are removed");
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": totp_at(&secret, step_at(now_unix()) + 1) })
        )
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "the authenticator application still works"
    );

    // The authenticator application removed (passkeys allowed instead, set in the database since
    // the API refuses it until MAIR-810): the agent's factor no longer counts.
    sqlx::query("UPDATE two_factor_policy SET totp_allowed = false, passkey_allowed = true")
        .execute(&env.raw)
        .await
        .unwrap();
    let (status, headers, ..) = login!(app, email);
    assert_eq!(
        status,
        StatusCode::OK,
        "optional policy: the password is enough"
    );
    assert!(headers.get("Authorization").is_some());
    sqlx::query("UPDATE two_factor_policy SET required = true")
        .execute(&env.raw)
        .await
        .unwrap();
    let (status, headers, body, _) = login!(app, email);
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(
        body["status"],
        json!("2fa_setup_required"),
        "mandatory policy: they must enrol another mode"
    );
    assert!(headers.get("Authorization").is_none());
}

#[actix_web::test]
async fn mair_808_a_mandatory_policy_without_encryption_key_refuses_the_login() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let redis = get_shared_redis().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), core_db_url().await).await);
    let raw = get_raw_pool(host).await;
    let (_, email) = create_account(&raw).await;
    sqlx::query("UPDATE two_factor_policy SET required = true")
        .execute(&raw)
        .await
        .unwrap();
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(public_config)
            .service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
    )
    .await;
    let (status, headers, ..) = login!(app, email);
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "never a session without the factor the policy requires"
    );
    assert!(headers.get("Authorization").is_none());
}

#[actix_web::test]
async fn mair_808_setup_refused_when_the_factor_got_active() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (id, email) = create_account(&env.raw).await;
    let (status, ..) = json_of!(app, put_policy(true, true, false, true, &admin));
    assert_eq!(status, StatusCode::OK);
    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();

    // The agent enrols through their own account meanwhile (another session, an administrator).
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    enrol!(app, auth);

    let (status, ..) = json_of!(
        app,
        post("/api/v1/auth/2fa/setup", &json!({ "token": token }))
    );
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the pending secret of an active factor is never replaced"
    );
}

// --- Review of MAIR-805 (CodeRabbit on the first pull request): one test per defect fixed. ---

fn login_key(token: &str) -> String {
    format!("2fa_login/{token}")
}

#[actix_web::test]
async fn mair_805_a_wrong_code_does_not_extend_the_login_token_lifetime() {
    let env = env().await;
    let redis = env.state.get_redis();
    let token = two_factor::begin_login(redis, 4242, "test").await.unwrap();
    // A token that is 200 seconds old: 100 seconds left of its five minutes.
    let aged = PendingLogin {
        user_id: 4242,
        device_info: "test".to_string(),
        failures: 0,
        expires_at: now_unix() + 100,
    };
    redis
        .set_ex(
            &login_key(&token),
            serde_json::to_string(&aged).unwrap(),
            100,
        )
        .await
        .unwrap();

    let burnt = two_factor::record_login_failure(redis, &token, aged.clone()).await;
    assert!(!burnt);
    let after = two_factor::read_login(redis, &token).await.unwrap();
    assert_eq!(after.failures, 1);
    assert_eq!(
        after.expires_at, aged.expires_at,
        "a wrong code must not give the token a new five minutes"
    );

    // Past its deadline the token is refused even while the key is still there, and a wrong
    // code on it burns it.
    let expired = PendingLogin {
        expires_at: now_unix() - 1,
        ..aged
    };
    redis
        .set_ex(
            &login_key(&token),
            serde_json::to_string(&expired).unwrap(),
            60,
        )
        .await
        .unwrap();
    assert!(two_factor::read_login(redis, &token).await.is_none());
    assert!(two_factor::record_login_failure(redis, &token, expired).await);
}

#[actix_web::test]
async fn mair_805_two_requests_cannot_both_spend_the_login_token() {
    let env = env().await;
    let redis = env.state.get_redis();
    let token = two_factor::begin_login(redis, 4243, "test").await.unwrap();
    let (a, b) = tokio::join!(
        two_factor::consume_login(redis, &token),
        two_factor::consume_login(redis, &token)
    );
    let winners = [a.unwrap(), b.unwrap()].iter().filter(|won| **won).count();
    assert_eq!(winners, 1, "exactly one request gets the session");
    assert!(two_factor::read_login(redis, &token).await.is_none());
    assert!(!two_factor::consume_login(redis, &token).await.unwrap());
}

#[actix_web::test]
async fn mair_805_the_step_and_the_confirmation_are_bound_to_the_secret_that_was_checked() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, _) = create_account(&env.raw).await;
    let uid = u64::try_from(id).unwrap();
    let auth = bearer(&env.state, uid).await;
    let db = env.state.get_smart_db();

    let (status, ..) = json_of!(
        app,
        test::TestRequest::post()
            .uri("/api/v1/user/me/2fa/setup/")
            .insert_header(auth.clone())
    );
    assert_eq!(status, StatusCode::OK);
    let stored: String = sqlx::query_scalar(
        "SELECT encode(secret_encrypted, 'base64') FROM user_totp WHERE user_id = $1",
    )
    .bind(id)
    .fetch_one(&env.raw)
    .await
    .unwrap();
    let other = general_purpose::STANDARD.encode([7u8; 48]);

    // A setup that replaced the pending secret after the code was checked must not be activated.
    let refused = db
        .fetch_one::<Written, _>(&ConfirmTotpQueryView::new(uid, 5, &other))
        .await;
    assert!(matches!(
        refused,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
    db.fetch_one::<Written, _>(&ConfirmTotpQueryView::new(uid, 5, &stored))
        .await
        .expect("the secret that was checked is the one activated");

    // A step spent for another secret (the factor was reset and enrolled again meanwhile) leaves
    // the anti-replay step of the current one alone.
    let refused = db
        .fetch_one::<Written, _>(&UseTotpStepQueryView::new(uid, 9, &other))
        .await;
    assert!(matches!(
        refused,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
    let last: Option<i64> =
        sqlx::query_scalar("SELECT last_used_step FROM user_totp WHERE user_id = $1")
            .bind(id)
            .fetch_one(&env.raw)
            .await
            .unwrap();
    assert_eq!(last, Some(5));
    db.fetch_one::<Written, _>(&UseTotpStepQueryView::new(uid, 9, &stored))
        .await
        .unwrap();
    let replay = db
        .fetch_one::<Written, _>(&UseTotpStepQueryView::new(uid, 9, &stored))
        .await;
    assert!(matches!(
        replay,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}

#[actix_web::test]
async fn mair_805_a_reset_waits_for_the_login_that_holds_the_factor() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, _) = create_account(&env.raw).await;
    let uid = u64::try_from(id).unwrap();
    let auth = bearer(&env.state, uid).await;
    enrol!(app, auth);
    let db = env.state.get_smart_db();

    // The second step takes this lock before it writes the session and holds it until commit.
    let mut login = db.begin().await.unwrap();
    login
        .fetch_one::<Written, _>(&LockActiveTotpQueryView::new(uid))
        .await
        .unwrap();
    let reset = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        db.execute(DeleteTotpQueryView::new(uid)),
    )
    .await;
    assert!(
        reset.is_err(),
        "the reset must wait: the session is committed first, then the reset revokes it"
    );
    login.commit().await.unwrap();
    db.execute(DeleteTotpQueryView::new(uid)).await.unwrap();

    // Once the factor is gone the lock finds nothing: the login is refused, no session.
    let mut late = db.begin().await.unwrap();
    let refused = late
        .fetch_one::<Written, _>(&LockActiveTotpQueryView::new(uid))
        .await;
    assert!(matches!(
        refused,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}

#[actix_web::test]
async fn mair_805_two_regenerations_leave_ten_backup_codes() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, _) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (_, codes) = enrol!(app, auth);

    let (first, second) = tokio::join!(
        async {
            json_of!(
                app,
                post(
                    "/api/v1/user/me/2fa/backup-codes/",
                    &json!({ "code": codes[0] })
                )
                .insert_header(auth.clone())
            )
        },
        async {
            json_of!(
                app,
                post(
                    "/api/v1/user/me/2fa/backup-codes/",
                    &json!({ "code": codes[1] })
                )
                .insert_header(auth.clone())
            )
        }
    );
    for (status, ..) in [&first, &second] {
        assert!(
            // The loser either replaced the winner's codes (200) or found its own code already
            // replaced (400, a refused code on this route).
            *status == StatusCode::OK || *status == StatusCode::BAD_REQUEST,
            "unexpected {status}"
        );
    }
    let usable: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM user_backup_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(id)
    .fetch_one(&env.raw)
    .await
    .unwrap();
    assert_eq!(usable, 10, "never two sets of codes at once");
}

#[actix_web::test]
async fn mair_805_a_retried_reset_publishes_the_revocations_the_first_one_missed() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, _) = create_account(&env.raw).await;
    let uid = u64::try_from(id).unwrap();
    let auth = bearer(&env.state, uid).await;
    enrol!(app, auth);

    // A session revoked by an earlier attempt whose publication to Redis failed: closed in the
    // table, absent from the revocation list, and no longer "active" for the revoke query.
    let missed = Uuid::new_v4();
    env.state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            missed,
            uid,
            &core_api::refresh_token::hash(&format!("2fa_{}", Uuid::new_v4())),
            "earlier_attempt",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1")
        .bind(missed)
        .execute(&env.raw)
        .await
        .unwrap();
    assert!(
        !is_session_revoked(env.state.get_redis(), &missed.to_string())
            .await
            .unwrap()
    );

    let admin = bearer(&env.state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()).await;
    let (status, ..) = json_of!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{id}/2fa"))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        is_session_revoked(env.state.get_redis(), &missed.to_string())
            .await
            .unwrap(),
        "the retry publishes the session the first attempt left out"
    );
}

#[actix_web::test]
async fn mair_805_a_password_reset_changes_nothing_when_the_second_factor_cannot_be_checked() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    enrol!(app, auth);
    let hash_before: String = sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&env.raw)
        .await
        .unwrap();

    // The same instance without `TOTP_ENCRYPTION_KEY`.
    let keyless = test::init_service(
        App::new()
            .app_data(env.state.clone())
            .configure(public_config)
            .service(
                web::scope("/api")
                    .wrap(actix_web::middleware::from_fn(session_guard))
                    .wrap(JwtMiddleware)
                    .configure(config),
            ),
    )
    .await;
    let reset_token = Uuid::new_v4().to_string();
    let key = format!("{reset_token}/forgot_password_email");
    env.state
        .get_redis()
        .secure_set_ex(&key, email.clone(), 900)
        .await
        .unwrap();
    let (status, headers, ..) = json_of!(
        keyless,
        post(
            "/api/v1/auth/reset_password",
            &json!({ "token": reset_token, "new_password": "Brand!New123", "device_info": "test" })
        )
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(headers.get("Authorization").is_none());

    let hash_after: String = sqlx::query_scalar("SELECT password FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&env.raw)
        .await
        .unwrap();
    assert_eq!(hash_before, hash_after, "the password is untouched");
    let open: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sessions WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .fetch_one(&env.raw)
    .await
    .unwrap();
    assert!(open >= 1, "the sessions are not revoked");
    assert!(
        env.state
            .get_redis()
            .secure_get::<String>(&key)
            .await
            .unwrap()
            .is_some(),
        "the reset token is kept: the agent retries once the instance is configured"
    );
}

#[actix_web::test]
async fn mair_806_a_spent_backup_code_has_its_own_answer_and_counts_as_a_failure() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let (id, email) = create_account(&env.raw).await;
    let auth = bearer(&env.state, u64::try_from(id).unwrap()).await;
    let (_, codes) = enrol!(app, auth);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[0] })
        )
    );
    assert_eq!(status, StatusCode::OK);

    let (_, _, body, _) = login!(app, email);
    let token = body["token"].as_str().unwrap().to_string();
    // A well-formed code that was never issued stays the generic refusal.
    let (status, _, _, text) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": "zzzz-zzzz" })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(text, "Invalid verification code.");
    // The spent code is told apart, in any typing of it, on every attempt up to the burn.
    for attempt in 2..=4 {
        let (status, _, _, text) = json_of!(
            app,
            post(
                "/api/v1/auth/login/2fa",
                &json!({ "token": token, "code": codes[0].to_uppercase() })
            )
        );
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {attempt}");
        assert_eq!(text, "Backup code already used.", "attempt {attempt}");
    }
    // The fifth failure burns the token, whatever its kind.
    let (status, _, _, text) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[0] })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(text, "Invalid or expired login token.");
    // An unspent backup code no longer works on the burnt token.
    let (status, ..) = json_of!(
        app,
        post(
            "/api/v1/auth/login/2fa",
            &json!({ "token": token, "code": codes[1] })
        )
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
