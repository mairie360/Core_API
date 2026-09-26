use crate::common::users::unique_marker;
use crate::common::{get_pool, get_raw_pool};
use core_api::database::auth::sso_login::{SsoLoginUserQueryResultView, SsoLoginUserQueryView};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::{
    get_shared_db, seed_password_hash, ALICE_ID, BOB_ID,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_sso_login_finds_active_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: SsoLoginUserQueryResultView = pool
        .fetch_one(&SsoLoginUserQueryView::new("alice@example.com"))
        .await
        .unwrap();

    assert_eq!(
        result,
        SsoLoginUserQueryResultView::new(*ALICE_ID.get().unwrap(), false)
    );
}

#[tokio::test]
#[serial]
async fn test_sso_login_ignores_email_case() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: SsoLoginUserQueryResultView = pool
        .fetch_one(&SsoLoginUserQueryView::new("Alice@Example.COM"))
        .await
        .unwrap();

    assert_eq!(result.user_id(), *ALICE_ID.get().unwrap());
}

#[tokio::test]
#[serial]
async fn test_sso_login_prefers_exact_case_match() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("ssocase");
    let email = format!("sso.{marker}@example.com");
    // Two accounts whose addresses only differ by case: Keycloak's exact spelling must win,
    // whichever was created first.
    for address in [email.to_uppercase(), email.clone()] {
        sqlx::query(
            "INSERT INTO users (first_name, last_name, email, password, status) \
             VALUES ('Sso', $1, $2, $3, 'active')",
        )
        .bind(&marker)
        .bind(&address)
        .bind(seed_password_hash())
        .execute(&raw)
        .await
        .unwrap();
    }
    let exact_id: i32 = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_one(&raw)
        .await
        .unwrap();

    let result: SsoLoginUserQueryResultView = pool
        .fetch_one(&SsoLoginUserQueryView::new(&email))
        .await
        .unwrap();

    assert_eq!(result.user_id(), exact_id);
}

#[tokio::test]
#[serial]
async fn test_sso_login_reports_archived_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: SsoLoginUserQueryResultView = pool
        .fetch_one(&SsoLoginUserQueryView::new("bob@example.com"))
        .await
        .unwrap();

    assert_eq!(
        result,
        SsoLoginUserQueryResultView::new(*BOB_ID.get().unwrap(), true)
    );
}

#[tokio::test]
#[serial]
async fn test_sso_login_unknown_email() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: Result<SsoLoginUserQueryResultView, _> = pool
        .fetch_one(&SsoLoginUserQueryView::new("stranger@danger.com"))
        .await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}

#[test]
fn test_sso_login_view_display_and_accessors() {
    let view = SsoLoginUserQueryView::new("alice@example.com");

    assert_eq!(view.email(), "alice@example.com");
    assert_eq!(
        view.to_string(),
        "SsoLoginUserQueryView: email = alice@example.com"
    );
}
