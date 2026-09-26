use crate::common::keycloak_mock::{KeycloakMock, CLIENT_ID, CLIENT_SECRET};
use actix_web::{http::StatusCode, test, web, App};
use core_api::endpoints::{config, public_config};
use core_api::keycloak::migration::{MigrationReport, UserMigrationStatus};
use core_api::keycloak::{KeycloakClient, KeycloakConfig};
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::{
    security::JwtMiddleware,
    state::AppState,
    test_setup::queries_setup::{get_shared_db, ADMIN_ID, ALICE_ID, BOB_ID, GROUP_OWNER_ID},
};
use serde_json::{json, Value};
use serial_test::serial;

const MIGRATION_PATH: &str = "/api/v1/admin/keycloak/migration";

// Same values as the other endpoint tests: one process, one JWT secret.
static INIT: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::env::set_var("JWT_SECRET", "refresh_test_secret");
    std::env::set_var("JWT_TIMEOUT", "3600");
});

async fn app_state() -> web::Data<AppState> {
    std::sync::LazyLock::force(&INIT);
    let (_container, host) = get_shared_db().await;
    web::Data::new(AppState::new(String::new(), host.clone()).await)
}

fn jwt_for(user_id: i32) -> String {
    generate_jwt(&user_id.to_string(), "").expect("test JWT")
}

// actix test services hold `Rc`s, so this future is `!Send`; tests run it on one thread anyway.
#[allow(clippy::future_not_send)]
async fn post_migration(
    state: &web::Data<AppState>,
    keycloak: Option<KeycloakClient>,
    jwt: Option<&str>,
    payload: Option<&Value>,
) -> (StatusCode, String) {
    let app = App::new().app_data(state.clone());
    let app = match keycloak {
        Some(keycloak) => app.app_data(web::Data::new(keycloak)),
        None => app,
    };
    let app = test::init_service(
        app.configure(public_config)
            .service(web::scope("/api").wrap(JwtMiddleware).configure(config)),
    )
    .await;
    let mut req = test::TestRequest::post().uri(MIGRATION_PATH);
    if let Some(jwt) = jwt {
        req = req.insert_header(("Authorization", format!("Bearer {jwt}")));
    }
    if let Some(payload) = payload {
        req = req.set_json(payload);
    }
    // The JWT and admin middlewares answer with an error instead of a response.
    match test::try_call_service(&app, req.to_request()).await {
        Ok(resp) => {
            let status = resp.status();
            let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
            (status, body)
        }
        Err(error) => (error.as_response_error().status_code(), error.to_string()),
    }
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_runs_for_an_admin() {
    let state = app_state().await;
    let mock = KeycloakMock::start();
    let admin_id = *ADMIN_ID.get().unwrap();

    let (status, body) =
        post_migration(&state, Some(mock.client()), Some(&jwt_for(admin_id)), None).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let report: MigrationReport = serde_json::from_str(&body).unwrap();
    assert!(report.total >= 4, "{report:?}");
    assert_eq!(report.users.len(), report.total);
    let alice = report
        .users
        .iter()
        .find(|result| result.user_id == *ALICE_ID.get().unwrap())
        .unwrap();
    assert_eq!(alice.status, UserMigrationStatus::Created);
    assert!(!alice.password_email_sent);
    let bob = report
        .users
        .iter()
        .find(|result| result.user_id == *BOB_ID.get().unwrap())
        .unwrap();
    assert!(!bob.enabled);
    assert_eq!(mock.users().len(), report.created);
    assert!(mock.password_emails().is_empty(), "default: no e-mail");

    // The JSON shape the BFF will read.
    let json: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["users"][0]["status"].as_str(), Some("created"));
    assert!(json["users"][0]["keycloak_id"].is_string());
    assert!(json["users"][0]["roles_added"].is_array());
    assert!(json["users"][0]["error"].is_null());
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_can_send_password_emails() {
    let state = app_state().await;
    let mock = KeycloakMock::start();

    let (status, body) = post_migration(
        &state,
        Some(mock.client()),
        Some(&jwt_for(*ADMIN_ID.get().unwrap())),
        Some(&json!({ "send_password_setup_email": true })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let report: MigrationReport = serde_json::from_str(&body).unwrap();
    let alice = report
        .users
        .iter()
        .find(|result| result.user_id == *ALICE_ID.get().unwrap())
        .unwrap();
    assert!(alice.password_email_sent);
    assert!(mock
        .password_emails()
        .contains(alice.keycloak_id.as_ref().unwrap()));
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_is_forbidden_to_non_admins() {
    let state = app_state().await;
    let mock = KeycloakMock::start();

    // The seeded group owner holds no role (Alice is an administrator).
    let (status, body) = post_migration(
        &state,
        Some(mock.client()),
        Some(&jwt_for(*GROUP_OWNER_ID.get().unwrap())),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, "Forbidden: User is not an admin.");
    assert!(mock.users().is_empty());
    assert!(mock.token_requests().is_empty());
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_requires_a_jwt() {
    let state = app_state().await;
    let mock = KeycloakMock::start();

    let (status, _) = post_migration(&state, Some(mock.client()), None, None).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(mock.users().is_empty());
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_without_keycloak_returns_503() {
    let state = app_state().await;

    let (status, body) =
        post_migration(&state, None, Some(&jwt_for(*ADMIN_ID.get().unwrap())), None).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body,
        "Keycloak migration is not configured: Keycloak sign-in must be enabled with a confidential client."
    );
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_with_a_public_client_returns_503() {
    let state = app_state().await;
    let mock = KeycloakMock::start();
    let public = KeycloakClient::new(mock.public_config());

    let (status, body) = post_migration(
        &state,
        Some(public),
        Some(&jwt_for(*ADMIN_ID.get().unwrap())),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(mock.token_requests().is_empty());
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_refused_service_account_returns_502() {
    let state = app_state().await;
    let mock = KeycloakMock::start();
    mock.deny_service_account();

    let (status, body) = post_migration(
        &state,
        Some(mock.client()),
        Some(&jwt_for(*ADMIN_ID.get().unwrap())),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body, "Keycloak refused Core's service account.");
}

#[tokio::test]
#[serial]
async fn test_keycloak_migration_unreachable_keycloak_returns_502() {
    let state = app_state().await;
    let unreachable = KeycloakClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        None,
        CLIENT_ID,
        Some(CLIENT_SECRET),
    ));

    let (status, body) = post_migration(
        &state,
        Some(unreachable),
        Some(&jwt_for(*ADMIN_ID.get().unwrap())),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body, "Keycloak is unavailable.");
}
