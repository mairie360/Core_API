//! MAIR-757: the password policy (CNIL 2022) on every route that sets a password. A password of
//! fewer than 12 characters, a commonly used one, or one containing the account's e-mail, first
//! name or last name answers `400` naming the rule; the one-time tokens stay usable after a
//! refusal so the agent can try again.

use crate::common::{core_db_url, get_pool, users::create_user, users::unique_marker};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::ADMIN_ID;
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::{json, Value};
use uuid::Uuid;

/// A password every rule accepts (and one containing the last name « Rivoire »), kept in constants so
/// the JSON bodies below carry no literal value after a `password` key (Gitleaks `generic-api-key`).
const COMPLIANT: &str = "Ecole-Jardin-Pont-2026";
const WITH_LAST_NAME: &str = "Rivoire-Mairie-2026";
const WITH_NEW_FIRST_NAME: &str = "Bellerive-Velo-2026";

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

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

macro_rules! call {
    ($app:expr, $req:expr) => {{
        let resp = test::call_service(&$app, $req.to_request()).await;
        let status = resp.status();
        let body = String::from_utf8_lossy(&test::read_body(resp).await).into_owned();
        (status, body)
    }};
}

fn admin_bearer() -> (&'static str, String) {
    std::sync::LazyLock::force(&INIT);
    let admin = ADMIN_ID.get().expect("seeded admin");
    let token = generate_jwt(&admin.to_string(), "admin").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

fn create(email: &str, password: &str) -> test::TestRequest {
    test::TestRequest::post()
        .uri("/api/v1/admin/users/")
        .insert_header(admin_bearer())
        .set_json(json!({
            "first_name": "Camille",
            "last_name": "Lefebvre",
            "email": email,
            "password": password,
        }))
}

#[tokio::test]
async fn mair_757_account_creation_names_the_broken_rule() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(
        AppState::new("redis://127.0.0.1:6379".to_string(), core_db_url().await).await,
    );
    let app = init_app!(state);
    let email = format!(
        "camille.lefebvre.{}@mairie.example.com",
        Uuid::new_v4().simple()
    );

    for (password, rule) in [
        ("Velo-2026-x", "at least 12 characters"),
        ("Unbelievable", "commonly used password"),
        ("Lefebvre-Mairie-2026", "last name"),
        ("CAMILLE-velo-2026", "first name"),
    ] {
        let (status, body) = call!(app, create(&email, password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{password}: {body}");
        assert!(body.contains(rule), "{password}: {body}");
    }
    let (status, body) = call!(app, create(&email, COMPLIANT));
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn mair_757_first_connection_change_refuses_the_name_and_keeps_the_token() {
    std::sync::LazyLock::force(&INIT);
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let user = create_user(&get_pool().await, "Gaspard", &unique_marker("force")).await;
    let token = Uuid::new_v4().to_string();
    state
        .get_redis()
        .set_ex(
            &format!("{token}/first_connection_id"),
            user.to_string().as_str(),
            900,
        )
        .await
        .unwrap();
    let change = |password: &str| {
        test::TestRequest::post()
            .uri("/api/v1/auth/force_change_password")
            .set_json(json!({ "token": token, "new_password": password }))
    };

    let (status, body) = call!(app, change("gaspard-de-la-mairie"));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("first name"), "{body}");
    let (status, body) = call!(app, change(COMPLIANT));
    assert_eq!(
        status,
        StatusCode::OK,
        "the token must survive the refusal: {body}"
    );
}

#[tokio::test]
async fn mair_757_password_reset_refuses_the_email_and_keeps_the_link() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let marker = unique_marker("reset");
    let user = create_user(&get_pool().await, "Hortense", &marker).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(&crate::common::get_raw_pool(host).await)
        .await
        .unwrap();
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    redis
        .set_ex(
            &format!("{token}/forgot_password_email"),
            email.as_str(),
            900,
        )
        .await
        .unwrap();
    redis
        .set_ex(
            &format!("{email}/forgot_password_token"),
            token.as_str(),
            900,
        )
        .await
        .unwrap();
    let reset = |password: String| {
        test::TestRequest::post()
            .uri("/api/v1/auth/reset_password")
            .set_json(json!({ "token": token, "new_password": password, "device_info": "test" }))
    };

    let local_part = email.split('@').next().unwrap().to_uppercase();
    let (status, body) = call!(app, reset(format!("{local_part}!2026")));
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("e-mail"), "{body}");
    let (status, body) = call!(app, reset(COMPLIANT.to_string()));
    assert_eq!(
        status,
        StatusCode::OK,
        "the link must survive the refusal: {body}"
    );
    let session: Value = serde_json::from_str(&body).unwrap();
    assert!(session["refresh_token"].is_string());
}

#[tokio::test]
async fn mair_757_administrator_reset_applies_the_policy() {
    std::sync::LazyLock::force(&INIT);
    let state = web::Data::new(
        AppState::new("redis://127.0.0.1:6379".to_string(), core_db_url().await).await,
    );
    let app = init_app!(state);
    let marker = unique_marker("admin");
    let user = create_user(&get_pool().await, "Isidore", &marker).await;
    let set = |password: &str| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/admin/users/{user}/password"))
            .insert_header(admin_bearer())
            .set_json(json!({ "new_password": password }))
    };

    for (password, rule) in [
        ("short-pass", "at least 12 characters"),
        ("Isidore-Mairie-2026", "first name"),
    ] {
        let (status, body) = call!(app, set(password));
        assert_eq!(status, StatusCode::BAD_REQUEST, "{password}: {body}");
        assert!(body.contains(rule), "{password}: {body}");
    }
    let (status, body) = call!(app, set(COMPLIANT));
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    // The account patch checks the identity the account will have: its new last name too.
    let patch = |body: Value| {
        test::TestRequest::patch()
            .uri(&format!("/api/v1/admin/users/{user}/"))
            .insert_header(admin_bearer())
            .set_json(body)
    };
    let (status, body) = call!(
        app,
        patch(json!({ "password": format!("{marker}-Mairie-2026") }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("last name"), "{body}");
    let (status, body) = call!(
        app,
        patch(json!({ "last_name": "Rivoire", "password": WITH_LAST_NAME }))
    );
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("last name"), "{body}");
    let (status, _) = call!(
        app,
        test::TestRequest::patch()
            .uri("/api/v1/admin/users/2147483000/password")
            .insert_header(admin_bearer())
            .set_json(json!({ "new_password": COMPLIANT }))
    );
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Review of MAIR-757: the identity rule is checked again inside the transaction that writes the password, on the account
/// row locked until the commit. While that transaction is open, a rename from another connection waits, so the password
/// can never be stored next to an identity it was not checked against.
#[tokio::test]
async fn mair_757_the_identity_check_locks_the_account_until_the_password_is_written() {
    use core_api::endpoints::password_identity::{check_in, IdentityChanges, IdentityCheckError};

    let smart_db = get_pool().await;
    let marker = unique_marker("lock");
    let user = u64::try_from(create_user(&smart_db, "Honorine", &marker).await).unwrap();

    let mut tx = smart_db.begin().await.unwrap();
    let refused = check_in(
        &mut tx,
        user,
        "new_password",
        "Honorine-Mairie-2026",
        IdentityChanges::default(),
    )
    .await;
    assert!(
        matches!(refused, Err(IdentityCheckError::Rejected(_))),
        "{refused:?}"
    );
    check_in(
        &mut tx,
        user,
        "new_password",
        COMPLIANT,
        IdentityChanges::default(),
    )
    .await
    .unwrap();

    let other = sqlx::postgres::PgPool::connect(&core_db_url().await)
        .await
        .unwrap();
    let mut rename = other.begin().await.unwrap();
    sqlx::query("SET LOCAL lock_timeout = '300ms'")
        .execute(&mut *rename)
        .await
        .unwrap();
    let blocked = sqlx::query("UPDATE users SET last_name = 'Bellerive' WHERE id = $1")
        .bind(i32::try_from(user).unwrap())
        .execute(&mut *rename)
        .await
        .expect_err("the rename must wait for the transaction that checked the identity");
    assert_eq!(
        blocked
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("55P03"),
        "{blocked}"
    );
    drop(rename);
    tx.rollback().await.unwrap();

    let missing = check_in(
        &mut smart_db.begin().await.unwrap(),
        2_147_483_000,
        "new_password",
        COMPLIANT,
        IdentityChanges::default(),
    )
    .await;
    assert_eq!(missing, Err(IdentityCheckError::UnknownUser));
}

/// Review of MAIR-757: a rename committed between the first identity check of a password reset and the transaction
/// that writes the password is caught by the locked check, and the reset link survives that refusal (it is only spent
/// with the write, just before the commit).
#[tokio::test]
async fn mair_757_a_rename_during_a_reset_is_refused_and_keeps_the_link() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let marker = unique_marker("rename");
    let user = create_user(&get_pool().await, "Hortense", &marker).await;
    let raw = crate::common::get_raw_pool(host).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(&raw)
        .await
        .unwrap();
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    redis
        .set_ex(
            &format!("{token}/forgot_password_email"),
            email.as_str(),
            900,
        )
        .await
        .unwrap();
    redis
        .set_ex(
            &format!("{email}/forgot_password_token"),
            token.as_str(),
            900,
        )
        .await
        .unwrap();
    let reset = |password: &str| {
        test::TestRequest::post()
            .uri("/api/v1/auth/reset_password")
            .set_json(json!({ "token": token, "new_password": password, "device_info": "test" }))
    };

    // Another connection renames the account and holds the row until it commits: the first check of the reset reads
    // the committed name (Hortense) and accepts the password, the locked check waits for the rename.
    let mut rename = raw.begin().await.unwrap();
    sqlx::query("UPDATE users SET first_name = 'Bellerive' WHERE id = $1")
        .bind(user)
        .execute(&mut *rename)
        .await
        .unwrap();
    let request = async { call!(app, reset(WITH_NEW_FIRST_NAME)) };
    let commit = async {
        // Commit only once the reset waits on the row lock: its first check has then read the old
        // name and passed, so the refusal can only come from the locked check.
        wait_for_a_lock_wait(&raw).await;
        rename.commit().await.unwrap();
    };
    let ((status, body), ()) = tokio::join!(request, commit);
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("first name"), "{body}");

    let (status, body) = call!(app, reset(COMPLIANT));
    assert_eq!(
        status,
        StatusCode::OK,
        "the link must survive the refusal: {body}"
    );
}

/// Waits until a session of the test database waits on a row lock (10 s at most).
async fn wait_for_a_lock_wait(raw: &sqlx::PgPool) {
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock'",
        )
        .fetch_one(raw)
        .await
        .unwrap();
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("no request ever waited on the account row");
}

/// Review of MAIR-757: the reset link is single-use even for two resets sent at once. Both pass the first checks; the
/// second one waits for the row lock of the first, then finds the link spent (`SET NX` marker) and answers `401`.
#[tokio::test]
async fn mair_757_two_resets_at_once_spend_the_link_once() {
    std::sync::LazyLock::force(&INIT);
    let host = crate::common::own_db_url().await;
    let (_redis, redis_config) = start_redis_container().await;
    let state = web::Data::new(AppState::new(redis_config.url.clone(), core_db_url().await).await);
    let app = init_app!(state);
    let marker = unique_marker("twice");
    let user = create_user(&get_pool().await, "Hortense", &marker).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(&crate::common::get_raw_pool(host).await)
        .await
        .unwrap();
    let token = Uuid::new_v4().to_string();
    let redis = state.get_redis();
    redis
        .set_ex(
            &format!("{token}/forgot_password_email"),
            email.as_str(),
            900,
        )
        .await
        .unwrap();
    redis
        .set_ex(
            &format!("{email}/forgot_password_token"),
            token.as_str(),
            900,
        )
        .await
        .unwrap();
    let reset = |password: &str| {
        test::TestRequest::post()
            .uri("/api/v1/auth/reset_password")
            .set_json(json!({ "token": token, "new_password": password, "device_info": "test" }))
    };

    let (first, second) = tokio::join!(async { call!(app, reset(COMPLIANT)) }, async {
        call!(app, reset(WITH_NEW_FIRST_NAME))
    });
    let mut statuses = [first.0, second.0];
    statuses.sort_unstable();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::UNAUTHORIZED],
        "{first:?} {second:?}"
    );
}
