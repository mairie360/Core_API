use crate::common::get_pool;
use core_api::database::auth::is_first_time::IsFirstTimeQueryView;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn is_first_time_existing_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = IsFirstTimeQueryView::new(*ALICE_ID.get().unwrap() as u64);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_ok(), "{:?}", result);
    assert!(result.unwrap());
}

#[tokio::test]
#[serial]
async fn is_first_time_unknown_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = IsFirstTimeQueryView::new(999_999);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_ok(), "{:?}", result);
    assert!(!result.unwrap());
}
