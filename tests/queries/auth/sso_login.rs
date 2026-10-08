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

/// Since Database 2.0.0 (`uq_users_email_lower`) two accounts cannot share an address that only
/// differs by case: the second one is refused, so a Keycloak e-mail designates one account
/// whatever its case. (The former test created both accounts to check that the exact spelling won,
/// which the schema no longer allows.)
#[tokio::test]
#[serial]
async fn test_sso_login_meets_one_account_per_address_whatever_its_case() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("ssocase");
    let email = format!("sso.{marker}@example.com");
    let insert = |address: String| {
        let raw = raw.clone();
        let marker = marker.clone();
        async move {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO users (first_name, last_name, email, password, status) \
                 VALUES ('Sso', $1, $2, $3, 'active') RETURNING id",
            )
            .bind(&marker)
            .bind(&address)
            .bind(seed_password_hash())
            .fetch_one(&raw)
            .await
        }
    };

    let account = insert(email.clone()).await.unwrap();
    let duplicate = insert(email.to_uppercase()).await.unwrap_err();
    let result: SsoLoginUserQueryResultView = pool
        .fetch_one(&SsoLoginUserQueryView::new(&email.to_uppercase()))
        .await
        .unwrap();

    assert_eq!(
        duplicate
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23505"),
        "{duplicate}"
    );
    assert_eq!(result.user_id(), account);
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
