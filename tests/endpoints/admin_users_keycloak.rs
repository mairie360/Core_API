//! Keycloak mirroring of the administration endpoints (MAIR-142): `POST /api/v1/admin/users/`,
//! `PATCH` / `DELETE /api/v1/admin/users/{id}/` and the role grant/revoke, against the fake
//! realm of `common::keycloak_mock`.

use crate::common::keycloak_mock::{KeycloakMock, CLIENT_ID, CLIENT_SECRET};
use crate::common::users::{link_identity, unique_marker, user_identities};
use crate::common::{get_pool, get_raw_pool};
use actix_web::http::Method;
use actix_web::{http::StatusCode, test, web, App};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::endpoints::{config, public_config};
use core_api::keycloak::migration::KEYCLOAK_PROVIDER;
use core_api::keycloak::{KeycloakAdminClient, KeycloakConfig};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::{
    security::JwtMiddleware,
    state::AppState,
    test_setup::queries_setup::{get_shared_db, ADMIN_ID},
};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::{PgPool, Row};

const USERS_PATH: &str = "/api/v1/admin/users/";
const TEST_ROLE: &str = "Keycloak Sync";

// Same values as the other endpoint tests: one process, one JWT secret.
static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

struct Env {
    state: web::Data<AppState>,
    pool: SmartDatabase,
    raw: PgPool,
}

async fn env() -> Env {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    Env {
        state: web::Data::new(AppState::new(String::new(), host.clone()).await),
        pool: get_pool(host.clone()).await,
        raw: get_raw_pool(host.clone()).await,
    }
}

fn admin_jwt() -> String {
    generate_jwt(&ADMIN_ID.get().unwrap().to_string(), "").expect("test JWT")
}

/// An admin client pointing at nothing: every Admin API call is `Unavailable`.
fn unreachable_admin() -> KeycloakAdminClient {
    KeycloakAdminClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        None,
        CLIENT_ID,
        Some(CLIENT_SECRET),
    ))
}

// actix test services hold `Rc`s, so this future is `!Send`; tests run it on one thread anyway.
#[allow(clippy::future_not_send)]
async fn call(
    env: &Env,
    admin: Option<KeycloakAdminClient>,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> (StatusCode, String) {
    let app = App::new().app_data(env.state.clone());
    let app = match admin {
        Some(admin) => app.app_data(web::Data::new(admin)),
        None => app,
    };
    let app = test::init_service(
        app.configure(public_config)
            .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
    )
    .await;
    let mut req = test::TestRequest::default()
        .method(method)
        .uri(path)
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt())));
    if let Some(payload) = payload {
        req = req.set_json(payload);
    }
    match test::try_call_service(&app, req.to_request()).await {
        Ok(resp) => {
            let status = resp.status();
            let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
            (status, body)
        }
        Err(error) => (error.as_response_error().status_code(), error.to_string()),
    }
}

/// A fresh active Core user, returned as `(id, email)`.
async fn fresh_user(env: &Env, first_name: &str) -> (i32, String) {
    let email = format!(
        "{}.{}@example.com",
        first_name.to_lowercase(),
        unique_marker("kc")
    );
    let _: bool = env
        .pool
        .fetch_scalar(&RegisterUserQueryView::new(
            first_name, "Synced", &email, "password", None,
        ))
        .await
        .unwrap();
    let id: i32 = env
        .pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();
    (id, email)
}

/// `(first_name, last_name, email, is_archived)` of a Core user.
async fn core_user(env: &Env, id: i32) -> (String, String, String, bool) {
    let row =
        sqlx::query("SELECT first_name, last_name, email, is_archived FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&env.raw)
            .await
            .unwrap();
    (row.get(0), row.get(1), row.get(2), row.get(3))
}

async fn core_user_exists(env: &Env, email: &str) -> bool {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
        .bind(email)
        .fetch_one(&env.raw)
        .await
        .unwrap()
}

/// Names of the Core roles of a user, sorted.
async fn core_roles(env: &Env, id: i32) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 ORDER BY r.name",
    )
    .bind(id)
    .fetch_all(&env.raw)
    .await
    .unwrap()
}

/// Id of the test role, created on first use.
async fn test_role_id(env: &Env) -> i32 {
    sqlx::query(
        "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, 'MAIR-142 tests', true) ON CONFLICT DO NOTHING",
    )
    .bind(TEST_ROLE)
    .execute(&env.raw)
    .await
    .unwrap();
    sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
        .bind(TEST_ROLE)
        .fetch_one(&env.raw)
        .await
        .unwrap()
}

async fn keycloak_subject(env: &Env, user_id: i32) -> Option<String> {
    user_identities(&env.raw, user_id)
        .await
        .into_iter()
        .find(|(provider, _)| provider == KEYCLOAK_PROVIDER)
        .map(|(_, subject)| subject)
}

fn create_body(email: &str) -> Value {
    json!({
        "first_name": "Claire",
        "last_name": "Martin",
        "email": email,
        "password": "MotDePasse!123",
        "phone_number": "0612345678"
    })
}

// --- POST /api/v1/admin/users/ ---------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_create_user_creates_links_and_invites_the_keycloak_account() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let email = format!("claire.{}@mairie360.test", unique_marker("create"));

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        USERS_PATH,
        Some(&create_body(&email)),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body, "User created successfully!");
    let keycloak_user = mock
        .user_by_email(&email)
        .expect("account created in the realm");
    assert_eq!(keycloak_user.first_name, "Claire");
    assert_eq!(keycloak_user.last_name, "Martin");
    assert!(keycloak_user.enabled);
    assert!(keycloak_user.email_verified);
    assert_eq!(
        mock.password_emails(),
        vec![keycloak_user.id.clone()],
        "the password set-up invitation is sent"
    );

    let user_id: i32 = env
        .pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();
    assert_eq!(
        keycloak_subject(&env, user_id).await.as_deref(),
        Some(keycloak_user.id.as_str()),
        "the Core account is linked to the realm account"
    );
    let roles = core_roles(&env, user_id).await;
    assert_eq!(
        keycloak_user.realm_roles, roles,
        "the default Core role is mapped as a realm role"
    );
}

#[tokio::test]
#[serial]
async fn test_create_user_adopts_an_existing_keycloak_account_without_inviting_it() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let email = format!("claire.{}@mairie360.test", unique_marker("adopt"));
    let existing = mock.seed_user(&email, false);

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        USERS_PATH,
        Some(&create_body(&email)),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(mock.users().len(), 1, "no duplicate account");
    let keycloak_user = mock.user(&existing).unwrap();
    assert_eq!(keycloak_user.first_name, "Claire", "profile overwritten");
    assert!(keycloak_user.enabled, "profile overwritten");
    assert!(
        mock.password_emails().is_empty(),
        "an adopted account already has its credentials"
    );
    let user_id: i32 = env
        .pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();
    assert_eq!(keycloak_subject(&env, user_id).await, Some(existing));
}

#[tokio::test]
#[serial]
async fn test_create_user_creates_nothing_when_keycloak_is_unreachable() {
    let env = env().await;
    let email = format!("claire.{}@mairie360.test", unique_marker("down"));

    let (status, body) = call(
        &env,
        Some(unreachable_admin()),
        Method::POST,
        USERS_PATH,
        Some(&create_body(&email)),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(body, "Keycloak is unavailable.");
    assert!(
        !core_user_exists(&env, &email).await,
        "nothing written to Core"
    );
}

#[tokio::test]
#[serial]
async fn test_create_user_deletes_the_reserved_account_when_the_invitation_fails() {
    let env = env().await;
    let mock = KeycloakMock::start();
    mock.fail_password_emails();
    let email = format!("claire.{}@mairie360.test", unique_marker("nomail"));

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        USERS_PATH,
        Some(&create_body(&email)),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(
        mock.users().is_empty(),
        "the reserved account is deleted again"
    );
    assert_eq!(mock.deletions().len(), 1);
    assert!(
        !core_user_exists(&env, &email).await,
        "nothing written to Core"
    );
}

#[tokio::test]
#[serial]
async fn test_create_user_refused_by_core_touches_nothing_in_keycloak() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (_, email) = fresh_user(&env, "Taken").await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        USERS_PATH,
        Some(&create_body(&email)),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body, "User already exists");
    assert!(mock.users().is_empty());
    assert!(
        mock.token_requests().is_empty(),
        "Keycloak is not even called"
    );
}

#[tokio::test]
#[serial]
async fn test_create_user_without_keycloak_writes_core_only() {
    let env = env().await;
    let email = format!("claire.{}@mairie360.test", unique_marker("nokc"));

    let (status, body) = call(
        &env,
        None,
        Method::POST,
        USERS_PATH,
        Some(&create_body(&email)),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    let user_id: i32 = env
        .pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();
    assert_eq!(keycloak_subject(&env, user_id).await, None);
}

// --- PATCH /api/v1/admin/users/{id}/ ---------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_patch_user_writes_the_new_profile_to_keycloak() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Patched").await;
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;
    let new_email = format!("renamed.{}@mairie360.test", unique_marker("patch"));

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::PATCH,
        &format!("{USERS_PATH}{user_id}/"),
        Some(&json!({ "first_name": "Renamed", "email": new_email })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let keycloak_user = mock.user(&keycloak_id).unwrap();
    assert_eq!(keycloak_user.first_name, "Renamed");
    assert_eq!(keycloak_user.last_name, "Synced", "kept from Core");
    assert_eq!(keycloak_user.email, new_email.to_lowercase());
    assert!(keycloak_user.email_verified);
    let (first_name, _, core_email, _) = core_user(&env, user_id).await;
    assert_eq!(first_name, "Renamed");
    assert_eq!(core_email, new_email);
}

#[tokio::test]
#[serial]
async fn test_patch_user_adopts_the_keycloak_account_by_email_when_not_linked_yet() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Unlinked").await;
    let keycloak_id = mock.seed_user(&email, true);

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::PATCH,
        &format!("{USERS_PATH}{user_id}/"),
        Some(&json!({ "last_name": "Adopted" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(mock.user(&keycloak_id).unwrap().last_name, "Adopted");
    assert_eq!(keycloak_subject(&env, user_id).await, Some(keycloak_id));
}

#[tokio::test]
#[serial]
async fn test_patch_user_restores_the_keycloak_profile_when_core_refuses() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Reverted").await;
    let (_, taken_email) = fresh_user(&env, "Other").await;
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::PATCH,
        &format!("{USERS_PATH}{user_id}/"),
        Some(&json!({ "first_name": "Renamed", "email": taken_email })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body, "Unknown user");
    let keycloak_user = mock.user(&keycloak_id).unwrap();
    assert_eq!(
        keycloak_user.email,
        email.to_lowercase(),
        "former profile restored"
    );
    assert_eq!(keycloak_user.first_name, "Reverted");
    let (first_name, _, core_email, _) = core_user(&env, user_id).await;
    assert_eq!(
        (first_name.as_str(), core_email.as_str()),
        ("Reverted", email.as_str())
    );
}

#[tokio::test]
#[serial]
async fn test_patch_user_unknown_to_keycloak_writes_core_only() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, _) = fresh_user(&env, "Local").await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::PATCH,
        &format!("{USERS_PATH}{user_id}/"),
        Some(&json!({ "first_name": "Renamed" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        mock.users().is_empty(),
        "no account created as a side effect"
    );
    assert_eq!(core_user(&env, user_id).await.0, "Renamed");
}

#[tokio::test]
#[serial]
async fn test_patch_user_core_only_fields_do_not_call_keycloak() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Phone").await;
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::PATCH,
        &format!("{USERS_PATH}{user_id}/"),
        Some(&json!({ "phone_number": "0798765432", "password": "NewPassword!123" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(mock.token_requests().is_empty(), "Keycloak is not called");
}

#[tokio::test]
#[serial]
async fn test_patch_user_changes_nothing_when_keycloak_is_unreachable() {
    let env = env().await;
    let (user_id, email) = fresh_user(&env, "Stuck").await;
    link_identity(&env.pool, user_id, "sub-stuck").await;

    let (status, body) = call(
        &env,
        Some(unreachable_admin()),
        Method::PATCH,
        &format!("{USERS_PATH}{user_id}/"),
        Some(&json!({ "first_name": "Renamed" })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    let (first_name, _, core_email, _) = core_user(&env, user_id).await;
    assert_eq!(
        (first_name.as_str(), core_email.as_str()),
        ("Stuck", email.as_str())
    );
}

// --- DELETE /api/v1/admin/users/{id}/ --------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_delete_user_disables_and_signs_out_the_keycloak_account() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Archived").await;
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::DELETE,
        &format!("{USERS_PATH}{user_id}/"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let keycloak_user = mock.user(&keycloak_id).expect("disabled, never deleted");
    assert!(!keycloak_user.enabled);
    assert_eq!(mock.logouts(), vec![keycloak_id]);
    assert!(core_user(&env, user_id).await.3, "archived in Core");
}

#[tokio::test]
#[serial]
async fn test_delete_user_re_enables_the_keycloak_account_when_core_refuses() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Owner").await;
    sqlx::query("INSERT INTO groups (owner_id, name) VALUES ($1, $2)")
        .bind(user_id)
        .bind(format!("Group {}", unique_marker("grp")))
        .execute(&env.raw)
        .await
        .unwrap();
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::DELETE,
        &format!("{USERS_PATH}{user_id}/"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, "User is already deleted");
    assert!(mock.user(&keycloak_id).unwrap().enabled, "re-enabled");
    assert!(!core_user(&env, user_id).await.3, "still active in Core");
}

#[tokio::test]
#[serial]
async fn test_delete_user_already_archived_does_not_call_keycloak() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, email) = fresh_user(&env, "Twice").await;
    let keycloak_id = mock.seed_user(&email, false);
    link_identity(&env.pool, user_id, &keycloak_id).await;
    crate::common::users::archive_user(&env.raw, user_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::DELETE,
        &format!("{USERS_PATH}{user_id}/"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(mock.token_requests().is_empty());
    assert!(mock.logouts().is_empty());
}

#[tokio::test]
#[serial]
async fn test_delete_user_changes_nothing_when_keycloak_is_unreachable() {
    let env = env().await;
    let (user_id, _) = fresh_user(&env, "Kept").await;
    link_identity(&env.pool, user_id, "sub-kept").await;

    let (status, body) = call(
        &env,
        Some(unreachable_admin()),
        Method::DELETE,
        &format!("{USERS_PATH}{user_id}/"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(!core_user(&env, user_id).await.3, "still active in Core");
}

// --- POST / DELETE /api/v1/admin/users/{id}/roles/ ------------------------------------------

#[tokio::test]
#[serial]
async fn test_grant_role_maps_the_realm_role() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let role_id = test_role_id(&env).await;
    let (user_id, email) = fresh_user(&env, "Granted").await;
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        &format!("{USERS_PATH}{user_id}/roles/"),
        Some(&json!({ "role_id": role_id, "user_id": user_id })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        mock.roles().iter().any(|role| role.name == TEST_ROLE),
        "realm role created"
    );
    assert_eq!(
        mock.user(&keycloak_id).unwrap().realm_roles,
        vec![TEST_ROLE.to_string()]
    );
    assert!(core_roles(&env, user_id)
        .await
        .contains(&TEST_ROLE.to_string()));
}

#[tokio::test]
#[serial]
async fn test_grant_role_unmaps_the_realm_role_when_core_refuses() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let role_id = test_role_id(&env).await;
    let (user_id, email) = fresh_user(&env, "Twice").await;
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(role_id)
        .execute(&env.raw)
        .await
        .unwrap();
    let keycloak_id = mock.seed_user(&email, true);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        &format!("{USERS_PATH}{user_id}/roles/"),
        Some(&json!({ "role_id": role_id, "user_id": user_id })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(
        mock.user(&keycloak_id).unwrap().realm_roles.is_empty(),
        "the mapping added by the call is removed"
    );
}

#[tokio::test]
#[serial]
async fn test_grant_role_unknown_role_answers_404_without_keycloak() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let (user_id, _) = fresh_user(&env, "NoRole").await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::POST,
        &format!("{USERS_PATH}{user_id}/roles/"),
        Some(&json!({ "role_id": 999_999, "user_id": user_id })),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(mock.token_requests().is_empty());
}

#[tokio::test]
#[serial]
async fn test_revoke_role_unmaps_the_realm_role() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let role_id = test_role_id(&env).await;
    let (user_id, email) = fresh_user(&env, "Revoked").await;
    sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(role_id)
        .execute(&env.raw)
        .await
        .unwrap();
    let keycloak_id = mock.seed_user(&email, true);
    mock.map_role(&keycloak_id, TEST_ROLE);
    mock.map_role(&keycloak_id, "offline_access");
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::DELETE,
        &format!("{USERS_PATH}{user_id}/roles/{role_id}"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert_eq!(
        mock.user(&keycloak_id).unwrap().realm_roles,
        vec!["offline_access".to_string()],
        "only the revoked role is unmapped"
    );
    assert!(!core_roles(&env, user_id)
        .await
        .contains(&TEST_ROLE.to_string()));
}

// Core never refuses a revoke (a role not held matches no row): the realm is aligned on Core.
#[tokio::test]
#[serial]
async fn test_revoke_role_not_held_in_core_unmaps_the_realm_role_anyway() {
    let env = env().await;
    let mock = KeycloakMock::start();
    let role_id = test_role_id(&env).await;
    let (user_id, email) = fresh_user(&env, "Unheld").await;
    let keycloak_id = mock.seed_user(&email, true);
    mock.map_role(&keycloak_id, TEST_ROLE);
    link_identity(&env.pool, user_id, &keycloak_id).await;

    let (status, body) = call(
        &env,
        Some(mock.admin_client()),
        Method::DELETE,
        &format!("{USERS_PATH}{user_id}/roles/{role_id}"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert!(
        mock.user(&keycloak_id).unwrap().realm_roles.is_empty(),
        "the realm mirrors Core"
    );
}

#[tokio::test]
#[serial]
async fn test_role_change_answers_502_and_changes_nothing_when_keycloak_is_unreachable() {
    let env = env().await;
    let role_id = test_role_id(&env).await;
    let (user_id, _) = fresh_user(&env, "Blocked").await;
    link_identity(&env.pool, user_id, "sub-blocked").await;

    let (status, body) = call(
        &env,
        Some(unreachable_admin()),
        Method::POST,
        &format!("{USERS_PATH}{user_id}/roles/"),
        Some(&json!({ "role_id": role_id, "user_id": user_id })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(body, "Keycloak is unavailable.");
    assert!(!core_roles(&env, user_id)
        .await
        .contains(&TEST_ROLE.to_string()));
}
