use crate::common::get_pool;
use core_api::database::groups::create_group::{create_group_query, CreateGroupQueryView};
use core_api::database::groups::delete_group::{delete_group_query, DeleteGroupQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn delete_group_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "delete_group_success_name",
        "delete_group_success_description",
    );
    let id = create_group_query(view, pool.clone()).await.unwrap();
    let view = DeleteGroupQueryView::new(id as u64);
    let result = delete_group_query(view, pool).await;
    assert!(result.is_ok(), "result should be Ok, got: {:?}", result);
}

#[tokio::test]
#[serial]
async fn delete_group_bad_group_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = DeleteGroupQueryView::new(999);
    let result = delete_group_query(view, pool).await;
    assert!(result.is_ok(), "result should be Ok, got: {:?}", result);
}
