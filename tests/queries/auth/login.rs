use crate::common::get_pool;
use core_api::database::auth::login::{login_query, LoginUserQueryResultView, LoginUserQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_login_user_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let result = login_query(
        LoginUserQueryView::new("alice@example.com".to_string(), "password123".to_string()),
        pool,
    )
    .await
    .unwrap();

    assert_eq!(
        result.unwrap(),
        LoginUserQueryResultView::new(1, "password123".to_string())
    );
}

#[tokio::test]
#[serial]
async fn test_login_user_wrong_password() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = login_query(
        LoginUserQueryView::new("alice@example.com".to_string(), "wrong_pass".to_string()),
        pool,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_login_user_unknown_email() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = login_query(
        LoginUserQueryView::new(
            "stranger@danger.com".to_string(),
            "any_password".to_string(),
        ),
        pool,
    )
    .await;

    assert_eq!(result, Ok(None));
}
