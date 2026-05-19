use crate::common::get_pool;
use core_api::database::groups::create_group::{create_group_query, CreateGroupQueryView};
use core_api::database::groups::get_group::{get_group_query, GetGroupQuerView, Group};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_group_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view =
        CreateGroupQueryView::new(1, "get_group_success_name", "get_group_success_description");
    let id = create_group_query(view, pool.clone()).await.unwrap();
    let group = Group::new(
        id,
        "get_group_success_name",
        1,
        Some("get_group_success_description"),
    );
    let view = GetGroupQuerView::new(id as u64);
    let result = get_group_query(view, pool.clone()).await;
    assert!(result.is_ok(), "result should be Ok, got: {:?}", result);
    let result = result.unwrap();
    assert_eq!(
        result, group,
        "result: {:#?}\nexpected: {:#?}",
        result, group
    );
}

#[tokio::test]
#[serial]
async fn get_group_bad_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetGroupQuerView::new(0);
    let result = get_group_query(view, pool.clone()).await;
    assert!(result.is_err(), "result should be Err, got: {:?}", result);
}
