use crate::common::get_pool;
use core_api::database::get_user_id::GetUserIdQueryView;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_id_success() {
    let pool = get_pool().await;

    let view = GetUserIdQueryView::new("alice@example.com");
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_ok(), "{result:?}");
    assert!(result.unwrap() > 0);
}

#[tokio::test]
#[serial]
async fn get_user_id_unknown_email() {
    let pool = get_pool().await;

    let view = GetUserIdQueryView::new("nobody-at-all@example.com");
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_err(), "{result:?}");
}
