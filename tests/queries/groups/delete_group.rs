use crate::common::get_pool;
use core_api::database::groups::create_group::CreateGroupQueryView;
use core_api::database::groups::delete_group::DeleteGroupQueryView;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn delete_group_success() {
    let pool = get_pool().await;

    let view = CreateGroupQueryView::new(
        1,
        "delete_group_success_name",
        "delete_group_success_description",
    );
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = DeleteGroupQueryView::new(id as u64);
    println!("{view}");
    let result = pool.execute(view).await;
    assert!(result.is_ok(), "result should be Ok, got: {result:?}");
}

#[tokio::test]
#[serial]
async fn delete_group_bad_group_id() {
    let pool = get_pool().await;
    let view = DeleteGroupQueryView::new(999);
    let result = pool.execute(view).await;
    assert!(result.is_ok(), "result should be Ok, got: {result:?}");
}
