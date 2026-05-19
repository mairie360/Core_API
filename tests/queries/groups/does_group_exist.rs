use crate::common::get_pool;
use core_api::database::groups::{
    create_group::{create_group_query, CreateGroupQueryView},
    does_group_exist::{does_group_exist_query, DoesGroupExistQuerView},
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn does_group_exist_true() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "does_group_exist_true_name",
        "does_group_exist_true_description",
    );
    let result = create_group_query(view, pool.clone()).await.unwrap();
    let view = DoesGroupExistQuerView::new(result as u64);
    let result = does_group_exist_query(view, pool).await;
    assert!(
        result.is_ok(),
        "group should exist, {result:?}",
        result = result
    );
    let bool_value = result.unwrap();
    assert!(
        bool_value,
        "result should be true, {bool_value:?}",
        bool_value = bool_value
    );
}

#[tokio::test]
#[serial]
async fn does_group_exist_false() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DoesGroupExistQuerView::new(999);
    let result = does_group_exist_query(view, pool).await;
    assert!(
        result.is_ok(),
        "result should be ok, {result:?}",
        result = result
    );
    let bool_value = result.unwrap();
    assert!(
        !bool_value,
        "result should be false, {result:?}",
        result = bool_value
    );
}
