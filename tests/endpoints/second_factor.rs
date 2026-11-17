//! MAIR-805: the second factor (TOTP and backup codes) managed by Core, end to end on the real
//! routes: enrolment, two-step login, replay and burn-out protections, disabling, administrator
//! reset, and the password reset that must not bypass it.

use crate::common::core_db_url;
use crate::common::get_raw_pool;
use actix_web::{http::StatusCode, test, web, App};
use base64::{engine::general_purpose, Engine as _};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::second_factor::{code_for_step, now_unix, step_at, SecretCipher};
use core_api::session_jwt::generate_session_jwt;
use data_encoding::BASE32_NOPAD;
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
        let (status, _, body, _) = json_of!(
            $app,
            test::TestRequest::post()
                .uri("/api/v1/user/me/2fa/setup/")
                .insert_header($auth.clone())
        );
        assert_eq!(status, StatusCode::OK);
        let secret = body["secret"].as_str().unwrap().to_string();
        let (status, _, body, _) = json_of!(
            $app,
            post("/api/v1/user/me/2fa/confirm/", &json!({ "code": totp_now(&secret) }))
                .insert_header($auth.clone())
        );
        assert_eq!(status, StatusCode::OK);
        let codes: Vec<String> = body["backup_codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap().to_string())
            .collect();
        (secret, codes)
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
    let (secret, _) = enrol!(app, auth);

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
            &json!({ "token": token, "code": totp_now(&secret) })
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
    let (status, ..) = json_of!(
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
