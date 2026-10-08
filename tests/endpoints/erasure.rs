//! MAIR-289: erasure (`POST /api/v1/admin/users/{userId}/erase`) and export
//! (`GET /api/v1/user/me/export`, `GET /api/v1/admin/users/{userId}/export`) of a user, on the
//! `anonymize_user()` / `export_user_data()` functions of Devops/Database (MAIR-289).
//!
//! The functions ship with Database #166 (release folder v3.0.1). Until a Database image with them
//! is published and pinned in `.cargo/config.toml` (`TEST_DB_VERSION`), these tests are ignored:
//! run them against a local build with
//! `TEST_DB_VERSION=<tag> cargo test --test integration_test erasure -- --ignored`.

use crate::common::{get_pool, get_raw_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use core_api::session_jwt::generate_session_jwt;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use mairie360_api_lib::test_setup::redis_setup::{
    get_redis_connection, start_redis_container, RedisTestConfig,
};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use redis::Commands;
use serde_json::Value;
use serial_test::serial;
use sqlx::Row;
use uuid::Uuid;

const NEEDS_DATABASE: &str =
    "needs a Database image with anonymize_user() / export_user_data() (Database #166, MAIR-289)";

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    // Same values as the other endpoint tests: they share the test binary.
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
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

struct Env {
    /// Keeps the Redis container alive for the test.
    _redis: Box<dyn std::any::Any>,
    redis: RedisTestConfig,
    state: web::Data<AppState>,
    host: String,
}

async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    let (node, redis) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis.url.clone(), host.clone()).await);
    Env {
        _redis: Box::new(node),
        redis,
        state,
        host: host.clone(),
    }
}

/// Opens a session for `user_id` as the login does; returns (session id, JWT).
async fn open_session(state: &AppState, user_id: u64) -> (Uuid, String) {
    let session_id = Uuid::new_v4();
    state
        .get_smart_db()
        .execute(CreateSessionQueryView::with_id(
            session_id,
            user_id,
            &core_api::refresh_token::hash(&format!("erasure_{session_id}")),
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();
    (
        session_id,
        generate_session_jwt(user_id, session_id).unwrap(),
    )
}

async fn fresh_user(host: &str, name: &str) -> u64 {
    let pool = get_pool(host.to_string()).await;
    u64::try_from(create_user(&pool, name, &unique_marker("erasure")).await).unwrap()
}

fn admin() -> u64 {
    u64::try_from(*ADMIN_ID.get().unwrap()).unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "needs a Database image with anonymize_user() / export_user_data() (Database #166, MAIR-289)"]
async fn admin_erasure_anonymizes_the_account_and_revokes_its_sessions() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "Erased").await;
    let (first, jwt) = open_session(&env.state, user).await;
    let (second, _) = open_session(&env.state, user).await;
    let (_, admin_jwt) = open_session(&env.state, admin()).await;
    let erase = |id: String| {
        test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{id}/erase"))
            .insert_header(("Authorization", format!("Bearer {admin_jwt}")))
            .to_request()
    };

    let resp = test::call_service(&app, erase(user.to_string())).await;
    assert_eq!(resp.status(), StatusCode::OK, "{NEEDS_DATABASE}");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["already_anonymized"], false);
    assert_eq!(body["revoked_sessions"], 2);
    assert_eq!(
        body["keycloak_account_deleted"], false,
        "no Keycloak in this test"
    );

    // The sessions are on the revocation list, the JWT is refused.
    let mut conn = get_redis_connection(&env.redis);
    for session in [first, second] {
        let exists: bool = conn.exists(format!("revoked:{session}")).unwrap();
        assert!(exists, "session {session} not published");
    }
    let me = test::TestRequest::get()
        .uri("/api/v1/user/me/")
        .insert_header(("Authorization", format!("Bearer {jwt}")))
        .to_request();
    // The middleware refuses it: an error, not a response.
    let refused = match test::try_call_service(&app, me).await {
        Ok(resp) => resp.status(),
        Err(err) => err.as_response_error().status_code(),
    };
    assert_eq!(refused, StatusCode::UNAUTHORIZED);

    // The row stays, archived and without identity.
    let row = sqlx::query("SELECT first_name, email, is_archived FROM users WHERE id = $1")
        .bind(i32::try_from(user).unwrap())
        .fetch_one(&get_raw_pool(env.host.clone()).await)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("first_name"), "Anonymized");
    assert!(row
        .get::<String, _>("email")
        .ends_with("@anonymized.invalid"));
    assert!(row.get::<bool, _>("is_archived"));

    // Twice changes nothing; the seeded administrator and unknown ids are refused.
    let again: Value =
        test::read_body_json(test::call_service(&app, erase(user.to_string())).await).await;
    assert_eq!(again["already_anonymized"], true);
    assert_eq!(
        // The seeded administrator (id 1), not the test admin: erasing it would break the tests
        // that share the database.
        test::call_service(&app, erase("1".into())).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        test::call_service(&app, erase("987654321".into()))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
#[serial]
#[ignore = "needs a Database image with anonymize_user() / export_user_data() (Database #166, MAIR-289)"]
async fn a_user_and_an_admin_export_the_data_of_the_account() {
    let env = env().await;
    let app = init_app!(env.state);
    let user = fresh_user(&env.host, "Exported").await;
    let (_, jwt) = open_session(&env.state, user).await;
    let (_, admin_jwt) = open_session(&env.state, admin()).await;
    let get = |uri: String, token: &str| {
        test::TestRequest::get()
            .uri(&uri)
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request()
    };

    let resp = test::call_service(&app, get("/api/v1/user/me/export".into(), &jwt)).await;
    assert_eq!(resp.status(), StatusCode::OK, "{NEEDS_DATABASE}");
    let own: Value = test::read_body_json(resp).await;
    assert_eq!(own["user"]["id"], user);
    assert!(
        own["user"].get("password").is_none(),
        "no password hash in the export"
    );
    assert!(own["data"]["sessions.user_id"]
        .as_array()
        .is_some_and(|s| !s.is_empty()));
    assert!(
        own["data"]["sessions.user_id"][0]
            .get("token_hash")
            .is_none(),
        "no token hash in the export"
    );

    let resp = test::call_service(
        &app,
        get(format!("/api/v1/admin/users/{user}/export"), &admin_jwt),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let by_admin: Value = test::read_body_json(resp).await;
    assert_eq!(by_admin["user"]["email"], own["user"]["email"]);

    let resp = test::call_service(
        &app,
        get("/api/v1/admin/users/987654321/export".into(), &admin_jwt),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
