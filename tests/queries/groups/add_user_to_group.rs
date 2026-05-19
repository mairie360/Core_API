use crate::common::get_pool;
use core_api::database::groups::{
    add_user_to_group::{add_user_to_group_query, AddUserToGroupQueryView},
    create_group::{create_group_query, CreateGroupQueryView},
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn add_user_to_group_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "add_user_to_group_name_success",
        "add_user_to_group_description_success",
    );
    let result = create_group_query(view, pool.clone()).await.unwrap();

    let view = AddUserToGroupQueryView::new(result as u64, 2);
    let result = add_user_to_group_query(view, pool).await;
    assert!(
        result.is_ok(),
        "add_user_to_group_success failed: {:?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn add_user_to_group_duplicate_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "add_user_to_group_duplicate_user_name",
        "add_user_to_group_duplicate_user_description",
    );
    let id = create_group_query(view, pool.clone()).await.unwrap();

    let view = AddUserToGroupQueryView::new(id as u64, 2);
    let _ = add_user_to_group_query(view, pool.clone()).await;
    let view = AddUserToGroupQueryView::new(id as u64, 2);
    let result = add_user_to_group_query(view, pool.clone()).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn add_user_to_group_unknow_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "add_user_to_group_unknow_user_name",
        "add_user_to_group_unknow_user_description",
    );
    let result = create_group_query(view, pool.clone()).await.unwrap();

    let view = AddUserToGroupQueryView::new(result as u64, 999);
    let result = add_user_to_group_query(view, pool).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn add_user_to_group_unknow_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = AddUserToGroupQueryView::new(999, 2);
    let result = add_user_to_group_query(view, pool).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn add_user_to_group_unknow_user_and_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = AddUserToGroupQueryView::new(999, 999);
    let result = add_user_to_group_query(view, pool).await;
    assert!(result.is_err());
}
