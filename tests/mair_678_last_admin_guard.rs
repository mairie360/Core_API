//! MAIR-678: an instance keeps at least one active administrator. Giving another role to the
//! last active one, revoking their `Admin` role or archiving them answers `409` and changes
//! nothing; with two administrators each can still demote or archive the other, and the check
//! and the write are serialised by a lock on the `Admin` role.
//!
//! Own test binary: these tests need to control how many administrators the database holds,
//! which the shared database of `integration_test` (where many fixtures are administrators) does
//! not allow. Cargo runs test binaries one after the other, each with its own container.
#![allow(clippy::cast_sign_loss)]

use actix_web::dev::ServiceResponse;
use actix_web::test::TestRequest;
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::last_admin::LAST_ADMIN_MESSAGE;
use core_api::endpoints::session_guard::session_guard;
use core_api::endpoints::{config, public_config};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::test_setup::queries_setup::{
    get_shared_db, get_shared_db_as, seed_password_hash, ADMIN_ID,
};
use mairie360_api_lib::{security::JwtMiddleware, state::AppState};
use serde_json::json;
use serial_test::serial;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "mair_678_last_admin_guard_secret");
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

/// Status and body of a response.
async fn read(resp: ServiceResponse) -> (StatusCode, String) {
    let status = resp.status();
    let body = test::read_body(resp).await;
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// The seeded administrator, who makes every call.
fn caller() -> i32 {
    *ADMIN_ID.get().expect("seeded admin")
}

fn bearer() -> (&'static str, String) {
    let token = generate_jwt(&caller().to_string(), "admin").unwrap();
    ("Authorization", format!("Bearer {token}"))
}

fn grant(user: i32, role: i32) -> TestRequest {
    TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{user}/roles/"))
        .insert_header(bearer())
        .set_json(json!({ "role_id": role }))
}

fn revoke(user: i32, role: i32) -> TestRequest {
    TestRequest::delete()
        .uri(&format!("/api/v1/admin/users/{user}/roles/{role}"))
        .insert_header(bearer())
}

fn archive(user: i32) -> TestRequest {
    TestRequest::delete()
        .uri(&format!("/api/v1/admin/users/{user}/"))
        .insert_header(bearer())
}

struct Fixture {
    state: web::Data<AppState>,
    raw: PgPool,
    admin_role: i32,
    user_role: i32,
}

/// Leaves the seeded administrator as the only active one: every other administrator becomes a
/// plain `User`.
async fn fixture() -> Fixture {
    std::sync::LazyLock::force(&INIT);
    let (_container, superuser_url) = get_shared_db().await;
    let raw = PgPoolOptions::new()
        .max_connections(5)
        .connect(superuser_url)
        .await
        .unwrap();
    let state =
        web::Data::new(AppState::new(String::new(), get_shared_db_as("core_api").await).await);
    let admin_role = role_id(&raw, "Admin").await;
    let user_role = role_id(&raw, "User").await;
    sqlx::query("UPDATE user_roles SET role_id = $1 WHERE role_id = $2 AND user_id <> $3")
        .bind(user_role)
        .bind(admin_role)
        .bind(caller())
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) \
         ON CONFLICT (user_id) DO UPDATE SET role_id = EXCLUDED.role_id",
    )
    .bind(caller())
    .bind(admin_role)
    .execute(&raw)
    .await
    .unwrap();
    assert_eq!(active_admins(&raw).await, vec![caller()]);
    Fixture {
        state,
        raw,
        admin_role,
        user_role,
    }
}

async fn role_id(raw: &PgPool, name: &str) -> i32 {
    sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
        .bind(name)
        .fetch_one(raw)
        .await
        .unwrap()
}

/// Ids of the active administrators, as `is_admin()` sees them.
async fn active_admins(raw: &PgPool) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT ur.user_id FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
         JOIN users u ON u.id = ur.user_id \
         WHERE r.name = 'Admin' AND u.is_archived IS NOT TRUE ORDER BY ur.user_id",
    )
    .fetch_all(raw)
    .await
    .unwrap()
}

/// Role names `user` holds.
async fn role_names(raw: &PgPool, user: i32) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id \
         WHERE ur.user_id = $1 ORDER BY r.id",
    )
    .bind(user)
    .fetch_all(raw)
    .await
    .unwrap()
}

async fn is_archived(raw: &PgPool, user: i32) -> bool {
    sqlx::query_scalar("SELECT COALESCE(is_archived, false) FROM users WHERE id = $1")
        .bind(user)
        .fetch_one(raw)
        .await
        .unwrap()
}

/// A fresh active account holding the `Admin` role.
async fn second_admin(fixture: &Fixture) -> i32 {
    let email = format!("mair678.{}@example.com", uuid::Uuid::new_v4().simple());
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO users (first_name, last_name, email, password, status) \
         VALUES ('Second', 'Admin', $1, $2, 'active') RETURNING id",
    )
    .bind(&email)
    .bind(seed_password_hash())
    .fetch_one(&fixture.raw)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) \
         ON CONFLICT (user_id) DO UPDATE SET role_id = EXCLUDED.role_id",
    )
    .bind(id)
    .bind(fixture.admin_role)
    .execute(&fixture.raw)
    .await
    .unwrap();
    id
}

/// The answer every refusal gives: `409` with the plain-text message the fronts display.
fn assert_last_admin_refusal((status, body): &(StatusCode, String)) {
    assert_eq!(*status, StatusCode::CONFLICT, "body: {body}");
    assert_eq!(body, LAST_ADMIN_MESSAGE);
}

#[tokio::test]
#[serial]
async fn mair_678_last_admin_cannot_be_given_another_role() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);

    let answer =
        read(test::call_service(&app, grant(caller(), fixture.user_role).to_request()).await).await;
    assert_last_admin_refusal(&answer);
    assert_eq!(role_names(&fixture.raw, caller()).await, vec!["Admin"]);
    assert_eq!(active_admins(&fixture.raw).await, vec![caller()]);

    // Granting `Admin` again is not a demotion: still the `409` of a role already held.
    let (status, body) =
        read(test::call_service(&app, grant(caller(), fixture.admin_role).to_request()).await)
            .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, "The user already holds this role.");
}

#[tokio::test]
#[serial]
async fn mair_678_last_admin_role_cannot_be_revoked() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);

    let answer =
        read(test::call_service(&app, revoke(caller(), fixture.admin_role).to_request()).await)
            .await;
    assert_last_admin_refusal(&answer);
    assert_eq!(role_names(&fixture.raw, caller()).await, vec!["Admin"]);

    // Revoking a role the last administrator does not hold changes nothing and is still allowed.
    let resp = test::call_service(&app, revoke(caller(), fixture.user_role).to_request()).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert_eq!(role_names(&fixture.raw, caller()).await, vec!["Admin"]);
}

#[tokio::test]
#[serial]
async fn mair_678_last_admin_cannot_be_archived() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);

    let answer = read(test::call_service(&app, archive(caller()).to_request()).await).await;
    assert_last_admin_refusal(&answer);
    assert!(!is_archived(&fixture.raw, caller()).await);
    assert_eq!(active_admins(&fixture.raw).await, vec![caller()]);
}

#[tokio::test]
#[serial]
async fn mair_678_archived_admin_does_not_count() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);
    // Holds `Admin` but archived: `is_admin()` is false for it.
    let archived = second_admin(&fixture).await;
    sqlx::query("UPDATE users SET is_archived = TRUE, status = 'archived' WHERE id = $1")
        .bind(archived)
        .execute(&fixture.raw)
        .await
        .unwrap();

    let answer =
        read(test::call_service(&app, grant(caller(), fixture.user_role).to_request()).await).await;
    assert_last_admin_refusal(&answer);
    assert_eq!(role_names(&fixture.raw, caller()).await, vec!["Admin"]);
}

#[tokio::test]
#[serial]
async fn mair_678_two_admins_can_demote_revoke_and_archive_each_other() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);
    let other = second_admin(&fixture).await;

    // Demoted by the other administrator.
    let resp = test::call_service(&app, grant(other, fixture.user_role).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(role_names(&fixture.raw, other).await, vec!["User"]);

    // Made administrator again through the API, then the `Admin` role revoked.
    let resp = test::call_service(&app, grant(other, fixture.admin_role).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = test::call_service(&app, revoke(other, fixture.admin_role).to_request()).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(role_names(&fixture.raw, other).await.is_empty());

    // Administrator again, then archived.
    let resp = test::call_service(&app, grant(other, fixture.admin_role).to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = test::call_service(&app, archive(other).to_request()).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(is_archived(&fixture.raw, other).await);

    // The caller is the last one again.
    let answer =
        read(test::call_service(&app, grant(caller(), fixture.user_role).to_request()).await).await;
    assert_last_admin_refusal(&answer);
}

/// The guard waits for the `Admin` role lock: a demotion running while another transaction holds
/// it (and demotes the other administrator) only decides once that transaction has committed,
/// so it sees the caller is now the last administrator. Without the lock both demotions passed.
#[tokio::test]
#[serial]
async fn mair_678_concurrent_demotions_cannot_remove_both_admins() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);
    let other = second_admin(&fixture).await;

    // A concurrent demotion of `other`, holding the lock the guarded writes take.
    let mut concurrent = fixture.raw.begin().await.unwrap();
    sqlx::query("SELECT id FROM roles WHERE name = 'Admin' FOR NO KEY UPDATE")
        .execute(&mut *concurrent)
        .await
        .unwrap();

    let demotion = test::call_service(&app, grant(caller(), fixture.user_role).to_request());
    tokio::pin!(demotion);
    assert!(
        tokio::time::timeout(Duration::from_millis(500), &mut demotion)
            .await
            .is_err(),
        "the demotion must wait for the Admin role lock"
    );

    sqlx::query("UPDATE user_roles SET role_id = $1 WHERE user_id = $2")
        .bind(fixture.user_role)
        .bind(other)
        .execute(&mut *concurrent)
        .await
        .unwrap();
    concurrent.commit().await.unwrap();

    let answer = read(demotion.await).await;
    assert_last_admin_refusal(&answer);
    assert_eq!(active_admins(&fixture.raw).await, vec![caller()]);
}

fn patch_role(role: i32, body: serde_json::Value) -> TestRequest {
    TestRequest::patch()
        .uri(&format!("/api/v1/admin/roles/{role}"))
        .insert_header(bearer())
        .set_json(body)
}

async fn role_can_be_deleted(raw: &PgPool, role: i32) -> Option<bool> {
    sqlx::query_scalar("SELECT can_be_deleted FROM roles WHERE id = $1")
        .bind(role)
        .fetch_one(raw)
        .await
        .unwrap()
}

/// The `Admin` role could be unprotected by `PATCH`ing `can_be_deleted`, then deleted or renamed,
/// which left the instance without any administrator despite the guards above.
#[tokio::test]
#[serial]
async fn mair_678_admin_role_cannot_be_unprotected_then_deleted() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);

    let (status, text) = read(
        test::call_service(
            &app,
            patch_role(fixture.admin_role, json!({ "can_be_deleted": true })).to_request(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{text}");
    assert_eq!(
        text,
        "System roles stay protected: can_be_deleted cannot be changed."
    );
    assert_eq!(
        role_can_be_deleted(&fixture.raw, fixture.admin_role).await,
        Some(false)
    );

    // Same through `PUT`, which replaces the role.
    let (status, text) = read(
        test::call_service(
            &app,
            TestRequest::put()
                .uri(&format!("/api/v1/admin/roles/{}", fixture.admin_role))
                .insert_header(bearer())
                .set_json(json!({
                    "name": "Admin",
                    "description": "Administrateur",
                    "can_be_deleted": true
                }))
                .to_request(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{text}");
    assert_eq!(
        text,
        "System roles stay protected: can_be_deleted cannot be changed."
    );
    assert_eq!(
        role_can_be_deleted(&fixture.raw, fixture.admin_role).await,
        Some(false)
    );

    // `null` reads as an omitted field: nothing is written, the role stays protected.
    let resp = test::call_service(
        &app,
        patch_role(fixture.admin_role, json!({ "can_be_deleted": null })).to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        role_can_be_deleted(&fixture.raw, fixture.admin_role).await,
        Some(false)
    );

    // So the deletion and the rename stay refused, and the administrator keeps the role.
    let resp = test::call_service(
        &app,
        TestRequest::delete()
            .uri(&format!("/api/v1/admin/roles/{}", fixture.admin_role))
            .insert_header(bearer())
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let resp = test::call_service(
        &app,
        patch_role(fixture.admin_role, json!({ "name": "Former admin" })).to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(active_admins(&fixture.raw).await, vec![caller()]);

    // Restating the protection and editing the description stay allowed.
    let resp = test::call_service(
        &app,
        patch_role(
            fixture.admin_role,
            json!({ "can_be_deleted": false, "description": "Administrateur de l'instance" }),
        )
        .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

/// A role created by the administrators can be protected; once protected it is a system role
/// like the base ones and cannot be unprotected any more.
#[tokio::test]
#[serial]
async fn mair_678_custom_role_once_protected_stays_protected() {
    let fixture = fixture().await;
    let app = init_app!(fixture.state);
    let role: i32 = sqlx::query_scalar(
        "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, 'MAIR-678', true) \
         RETURNING id",
    )
    .bind(format!("mair678-{}", uuid::Uuid::new_v4().simple()))
    .fetch_one(&fixture.raw)
    .await
    .unwrap();

    // Writing `true` on a deletable role is still accepted.
    for (flag, status, stored) in [
        (true, StatusCode::OK, Some(true)),
        (false, StatusCode::OK, Some(false)),
        (true, StatusCode::FORBIDDEN, Some(false)),
    ] {
        let resp = test::call_service(
            &app,
            patch_role(role, json!({ "can_be_deleted": flag })).to_request(),
        )
        .await;
        assert_eq!(resp.status(), status, "can_be_deleted = {flag}");
        assert_eq!(role_can_be_deleted(&fixture.raw, role).await, stored);
    }
}
