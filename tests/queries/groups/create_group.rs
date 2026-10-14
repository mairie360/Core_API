use crate::common::get_pool;
use core_api::database::groups::create_group::CreateGroupQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn create_group_success() {
    let pool = get_pool().await;

    let view = CreateGroupQueryView::new(
        1,
        "create_group_name_success",
        "create_group_description_success",
    );
    println!("{view}");
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn create_group_duplicate_name() {
    let pool = get_pool().await;

    let view = CreateGroupQueryView::new(
        1,
        "create_group_name_duplicate",
        "create_group_description_duplicate",
    );
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_ok());

    // The endpoint maps this unique violation to 409.
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::UniqueViolation(_)))
    ));
}
