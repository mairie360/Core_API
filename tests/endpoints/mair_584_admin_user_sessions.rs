//! MAIR-584 / MAIR-904 (epic MAIR-824, V2 of the administration): an administrator closes one or
//! every session of a user, suspends and reactivates an account, and the admin list tells how
//! many sessions a user has open, their last sign-in and their second factor.

use crate::common::{core_db_url, get_raw_pool};
use actix_web::{http::StatusCode, test, web, App};
use base64::{engine::general_purpose, Engine as _};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::second_factor::SecretCipher;
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::jwt_manager::is_session_revoked;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::get_shared_redis;
use mairie360_api_lib::{password::hash_password, security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    mairie360_api_lib::test_setup::http_setup::init_test_jwt_env();
    std::env::set_var("JWT_TIMEOUT", "3600");
});

const PASSWORD: &str = "Correct!Pass123";

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

macro_rules! call {
    ($app:expr, $req:expr) => {{
        match test::try_call_service(&$app, $req.to_request()).await {
            Ok(resp) => {
                let status = resp.status();
                let bytes = test::read_body(resp).await;
                let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                (status, body, String::from_utf8_lossy(&bytes).to_string())
            }
            Err(err) => (
                err.as_response_error().status_code(),
                Value::Null,
                err.to_string(),
            ),
        }
    }};
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
        SecretCipher::from_base64(&general_purpose::STANDARD.encode([7u8; 32])).unwrap(),
    );
    Env {
        state,
        cipher,
        raw: get_raw_pool(host).await,
    }
}

/// Inserts an active account with `PASSWORD` and returns `(id, email)`.
async fn create_account(raw: &PgPool) -> (u64, String) {
    let email = format!("mair584.{}@example.com", Uuid::new_v4());
    let id: i32 = sqlx::query(
        "INSERT INTO users (first_name, last_name, email, password, status, first_connect, \
         is_archived) VALUES ('Session', 'Holder', $1, $2, 'active', false, false) RETURNING id",
    )
    .bind(&email)
    .bind(hash_password(PASSWORD).unwrap())
    .fetch_one(raw)
    .await
    .unwrap()
    .get("id");
    (u64::try_from(id).unwrap(), email)
}

/// Opens a session for `user_id` as the login does; returns `(session id, Authorization header)`.
async fn open_session(state: &AppState, user_id: u64) -> (Uuid, (&'static str, String)) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&format!("mair584_{}", Uuid::new_v4())),
            "Firefox 140 sur Linux",
            std::net::IpAddr::from([203, 0, 113, 24]),
        ))
        .await
        .unwrap();
    let jwt = generate_session_jwt(user_id, session_id, "").unwrap();
    (session_id, ("Authorization", format!("Bearer {jwt}")))
}

async fn admin(state: &AppState) -> (&'static str, String) {
    open_session(state, u64::try_from(*ADMIN_ID.get().unwrap()).unwrap())
        .await
        .1
}

fn me(auth: &(&'static str, String)) -> test::TestRequest {
    test::TestRequest::get()
        .uri("/api/v1/user/me/")
        .insert_header(auth.clone())
}

fn login(email: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": email, "password": PASSWORD, "device_info": "test"
        }))
}

/// The row of `email` in the admin list.
macro_rules! listed {
    ($app:expr, $admin:expr, $email:expr) => {{
        let (status, body, _) = call!(
            $app,
            test::TestRequest::get()
                .uri(&format!("/api/v1/admin/users/?search={}", $email))
                .insert_header($admin.clone())
        );
        assert_eq!(status, StatusCode::OK);
        body["users"][0].clone()
    }};
}

#[actix_web::test]
async fn mair_584_the_admin_list_counts_the_active_sessions_and_the_last_sign_in() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (id, email) = create_account(&env.raw).await;

    let row = listed!(app, admin, email);
    assert_eq!(row["active_sessions"], 0);
    assert!(row["last_login_at"].is_null());
    assert_eq!(
        row["two_factor"],
        json!({ "methods": [], "last_verified_at": null })
    );

    let (first, _) = open_session(&env.state, id).await;
    open_session(&env.state, id).await;
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1")
        .bind(first)
        .execute(&env.raw)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO user_totp (user_id, secret_encrypted, confirmed_at, last_used_step) \
         VALUES ($1, '\\x00'::bytea, now(), 59000000)",
    )
    .bind(i32::try_from(id).unwrap())
    .execute(&env.raw)
    .await
    .unwrap();

    let row = listed!(app, admin, email);
    assert_eq!(
        row["active_sessions"], 1,
        "a revoked session is not counted"
    );
    let last_login = row["last_login_at"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(last_login).is_ok(),
        "{last_login}"
    );
    assert_eq!(row["two_factor"]["methods"], json!(["totp"]));
    assert_eq!(
        row["two_factor"]["last_verified_at"], "2026-02-02T02:40:00Z",
        "step 59000000 × 30 s"
    );
}

#[actix_web::test]
async fn mair_584_an_administrator_closes_one_session_of_a_user() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (id, _) = create_account(&env.raw).await;
    let (closed, closed_auth) = open_session(&env.state, id).await;
    let (_, kept_auth) = open_session(&env.state, id).await;

    let uri = format!("/api/v1/admin/users/{id}/sessions/{closed}");
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri(&uri)
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(call!(app, me(&closed_auth)).0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        call!(app, me(&kept_auth)).0,
        StatusCode::OK,
        "the other session stays open"
    );
    assert!(
        is_session_revoked(env.state.get_redis(), &closed.to_string())
            .await
            .unwrap()
    );

    // Already closed, another user's session, unknown user, not an id: refused, nothing changes.
    let (status, _, text) = call!(
        app,
        test::TestRequest::delete()
            .uri(&uri)
            .insert_header(admin.clone())
    );
    assert_eq!(
        (status, text.as_str()),
        (StatusCode::NOT_FOUND, "Unknown session")
    );
    let (other_id, _) = create_account(&env.raw).await;
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{other_id}/sessions/{closed}"))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, text) = call!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/99999999/sessions/{closed}"))
            .insert_header(admin.clone())
    );
    assert_eq!(
        (status, text.as_str()),
        (StatusCode::NOT_FOUND, "Unknown user")
    );
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{id}/sessions/not-a-session"))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A plain user cannot close a session, even their own through this route.
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/admin/users/{id}/sessions"))
            .insert_header(kept_auth)
    );
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn mair_584_an_administrator_closes_every_session_of_a_user() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (id, email) = create_account(&env.raw).await;
    let (first, first_auth) = open_session(&env.state, id).await;
    let (second, second_auth) = open_session(&env.state, id).await;

    let uri = format!("/api/v1/admin/users/{id}/sessions");
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri(&uri)
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(call!(app, me(&first_auth)).0, StatusCode::UNAUTHORIZED);
    assert_eq!(call!(app, me(&second_auth)).0, StatusCode::UNAUTHORIZED);
    for sid in [first, second] {
        assert!(is_session_revoked(env.state.get_redis(), &sid.to_string())
            .await
            .unwrap());
    }
    assert_eq!(listed!(app, admin, email)["active_sessions"], 0);

    // Idempotent; the account can sign in again; unknown user is a 404.
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri(&uri)
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(call!(app, login(&email)).0, StatusCode::OK);
    let (status, ..) = call!(
        app,
        test::TestRequest::delete()
            .uri("/api/v1/admin/users/99999999/sessions")
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn mair_904_a_suspended_account_loses_its_sessions_and_cannot_sign_in_until_reactivated() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (id, email) = create_account(&env.raw).await;
    let (sid, auth) = open_session(&env.state, id).await;

    let suspend = format!("/api/v1/admin/users/{id}/suspend");
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&suspend)
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(call!(app, me(&auth)).0, StatusCode::UNAUTHORIZED);
    assert!(is_session_revoked(env.state.get_redis(), &sid.to_string())
        .await
        .unwrap());
    assert_eq!(call!(app, login(&email)).0, StatusCode::UNAUTHORIZED);
    let row = listed!(app, admin, email);
    assert_eq!(
        (row["status"].clone(), row["active_sessions"].clone()),
        (json!("suspended"), json!(0))
    );
    // The role is kept for the reactivation.
    assert!(row["role"].is_object());

    // Idempotent.
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&suspend)
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);

    let reactivate = format!("/api/v1/admin/users/{id}/reactivate");
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&reactivate)
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(listed!(app, admin, email)["status"], "offline");
    assert_eq!(call!(app, login(&email)).0, StatusCode::OK);
    // Reactivating an account that is not suspended changes nothing.
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&reactivate)
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn mair_904_suspension_refusals() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin_id = u64::try_from(*ADMIN_ID.get().unwrap()).unwrap();
    let admin = admin(&env.state).await;

    // Own account: 409, the administrator stays active.
    let (status, _, text) = call!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{admin_id}/suspend"))
            .insert_header(admin.clone())
    );
    assert_eq!(
        (status, text.as_str()),
        (StatusCode::CONFLICT, "You cannot suspend your own account.")
    );
    assert_eq!(call!(app, me(&admin)).0, StatusCode::OK);

    // Unknown and archived accounts: 404.
    for path in ["suspend", "reactivate"] {
        let (status, ..) = call!(
            app,
            test::TestRequest::post()
                .uri(&format!("/api/v1/admin/users/99999999/{path}"))
                .insert_header(admin.clone())
        );
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    let (archived, _) = create_account(&env.raw).await;
    crate::common::users::archive_user(&env.raw, i32::try_from(archived).unwrap()).await;
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{archived}/suspend"))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A plain user cannot suspend anyone.
    let (id, _) = create_account(&env.raw).await;
    let (_, plain) = open_session(&env.state, id).await;
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{admin_id}/suspend"))
            .insert_header(plain)
    );
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn mair_904_a_suspended_administrator_loses_the_administration() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (id, _) = create_account(&env.raw).await;
    sqlx::query(
        "UPDATE user_roles SET role_id = (SELECT id FROM roles WHERE name = 'Admin') WHERE user_id = $1",
    )
    .bind(i32::try_from(id).unwrap())
    .execute(&env.raw)
    .await
    .unwrap();
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{id}/suspend"))
            .insert_header(admin)
    );
    assert_eq!(status, StatusCode::NO_CONTENT);
    // Even with a session opened behind Core's back, the account is refused: since MAIR-772 the JWT
    // middleware asks `session_state()`, which reads a session of a suspended account as inactive (401)
    // before the administration guard would answer 403.
    let (_, auth) = open_session(&env.state, id).await;
    let (status, ..) = call!(
        app,
        test::TestRequest::get()
            .uri("/api/v1/admin/users/")
            .insert_header(auth)
    );
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn mair_904_the_admin_list_filters_by_account_status_and_refuses_unknown_filters() {
    let env = env().await;
    let app = init_app!(env.state, env.cipher);
    let admin = admin(&env.state).await;
    let (id, email) = create_account(&env.raw).await;
    let (status, ..) = call!(
        app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{id}/suspend"))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body, _) = call!(
        app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/users/?search={email}&account_status=suspended&sort=-name"
            ))
            .insert_header(admin.clone())
    );
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["users"][0]["account_status"], "suspended");
    let (_, body, _) = call!(
        app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/users/?search={email}&account_status=active"
            ))
            .insert_header(admin.clone())
    );
    assert_eq!(body["total"], 0);

    for query in ["account_status=deleted", "sort=email"] {
        let (status, _, text) = call!(
            app,
            test::TestRequest::get()
                .uri(&format!("/api/v1/admin/users/?{query}"))
                .insert_header(admin.clone())
        );
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {text}");
    }
}
