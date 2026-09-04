use crate::common::get_pool;
use core_api::database::groups::{
    add_user_to_group::AddUserToGroupQueryView, create_group::CreateGroupQueryView,
    delete_user_from_group::DeleteUserFromGroupQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn delete_user_to_group_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "delete_user_to_group_name_success",
        "delete_user_to_group_description_success",
    );
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();

    let view = AddUserToGroupQueryView::new(result as u64, 2);
    let _ = pool.execute(view).await;
    let view = DeleteUserFromGroupQueryView::new(result as u64, 2);
    println!("{}", view);
    let result = pool.execute(view).await;
    assert!(
        result.is_ok(),
        "delete_user_from_group_query should succeed, {result:?}",
        result = result,
    );
}

#[tokio::test]
#[serial]
async fn delete_user_to_group_unknow_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteUserFromGroupQueryView::new(999, 2);
    let result = pool.execute(view).await;
    assert!(
        result.is_ok(),
        "delete_user_from_group_query should succeed, {result:?}",
        result = result,
    );
}

#[tokio::test]
#[serial]
async fn delete_user_to_group_unknow_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteUserFromGroupQueryView::new(1, 999);
    let result = pool.execute(view).await;
    assert!(
        result.is_ok(),
        "delete_user_from_group_query should succeed, {result:?}",
        result = result,
    );
}

#[tokio::test]
#[serial]
async fn delete_user_to_group_unknow_user_and_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteUserFromGroupQueryView::new(999, 999);
    let result = pool.execute(view).await;
    assert!(
        result.is_ok(),
        "delete_user_from_group_query should succeed, {result:?}",
        result = result,
    );
}
