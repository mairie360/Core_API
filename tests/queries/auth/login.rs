use crate::common::get_pool;
use core_api::database::auth::login::{LoginUserQueryResultView, LoginUserQueryView};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_login_user_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let result: LoginUserQueryResultView = pool
        .fetch_one(&LoginUserQueryView::new(
            "alice@example.com".to_string(),
            "password123".to_string(),
        ))
        .await
        .unwrap();

    assert_eq!(
        result,
        LoginUserQueryResultView::new(1, "password123".to_string(), true)
    );
}

#[tokio::test]
#[serial]
async fn test_login_user_wrong_password() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result: Result<LoginUserQueryResultView, _> = pool
        .fetch_one(&LoginUserQueryView::new(
            "alice@example.com".to_string(),
            "wrong_pass".to_string(),
        ))
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_login_user_unknown_email() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result: Result<LoginUserQueryResultView, _> = pool
        .fetch_one(&LoginUserQueryView::new(
            "stranger@danger.com".to_string(),
            "any_password".to_string(),
        ))
        .await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}
