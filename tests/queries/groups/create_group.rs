use crate::common::get_pool;
use core_api::database::groups::create_group::CreateGroupQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn create_group_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "create_group_name_success",
        "create_group_description_success",
    );
    println!("{}", view);
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn create_group_duplicate_name() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "create_group_name_duplicate",
        "create_group_description_duplicate",
    );
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_ok());

    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_err());
}
