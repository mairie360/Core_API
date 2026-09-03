use crate::common::get_pool;
use core_api::database::get_user_id::GetUserIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_id_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserIdQueryView::new("alice@example.com");
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_ok(), "{:?}", result);
    assert!(result.unwrap() > 0);
}

#[tokio::test]
#[serial]
async fn get_user_id_unknown_email() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserIdQueryView::new("nobody-at-all@example.com");
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_err(), "{:?}", result);
}
