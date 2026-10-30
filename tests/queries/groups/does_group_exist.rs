use crate::common::get_pool;
use core_api::database::groups::{
    create_group::CreateGroupQueryView, does_group_exist::DoesGroupExistQuerView,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn does_group_exist_true() {
    let pool = get_pool().await;

    let view = CreateGroupQueryView::new(
        1,
        "does_group_exist_true_name",
        "does_group_exist_true_description",
    );
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = DoesGroupExistQuerView::new(result as u64);
    println!("{view}");
    let result = pool.fetch_scalar(&view).await;
    assert!(result.is_ok(), "group should exist, {result:?}");
    let bool_value: bool = result.unwrap();
    assert!(bool_value, "result should be true, {bool_value:?}");
}

#[tokio::test]
#[serial]
async fn does_group_exist_false() {
    let pool = get_pool().await;

    let view = DoesGroupExistQuerView::new(999);
    let result = pool.fetch_scalar(&view).await;
    assert!(result.is_ok(), "result should be ok, {result:?}");
    let bool_value: bool = result.unwrap();
    assert!(!bool_value, "result should be false, {bool_value:?}");
}
