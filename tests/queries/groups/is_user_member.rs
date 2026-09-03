use crate::common::get_pool;
use core_api::database::groups::{
    add_user_to_group::AddUserToGroupQueryView, create_group::CreateGroupQueryView,
    is_user_member::IsUserMemberQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn is_user_member_true() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "is_user_member_true_name",
        "is_user_member_true_description",
    );
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();

    let view = AddUserToGroupQueryView::new(result as u64, 2);
    let _ = pool.execute(view).await;
    let view = IsUserMemberQueryView::new(result as u64, 2);
    println!("{}", view);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_ok(), "is_user_member_true failed: {:?}", result);
    assert!(
        result.unwrap(),
        "is_user_member_true failed: result is false"
    );
}

#[tokio::test]
#[serial]
async fn is_user_member_false() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "is_user_member_false_name",
        "is_user_member_false_description",
    );
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();

    let view = IsUserMemberQueryView::new(id as u64, 3);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_ok(), "is_user_member_false failed: {:?}", result);
    assert!(
        !result.unwrap(),
        "is_user_member_false failed: result is true"
    );
}

#[tokio::test]
#[serial]
async fn is_user_member_unknow_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateGroupQueryView::new(
        1,
        "is_user_member_unknow_user_name",
        "is_user_member_unknow_user_description",
    );
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();

    let view = IsUserMemberQueryView::new(result as u64, 999);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;
    assert!(
        result.is_ok(),
        "is_user_member_unknow_user failed: {:?}",
        result
    );
    assert!(
        !result.unwrap(),
        "is_user_member_unknow_user failed: result is true"
    );
}

#[tokio::test]
#[serial]
async fn is_user_member_unknow_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = IsUserMemberQueryView::new(999, 2);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;
    assert!(
        result.is_ok(),
        "is_user_member_unknow_group failed: {:?}",
        result
    );
    assert!(
        !result.unwrap(),
        "is_user_member_unknow_group failed: result is true"
    );
}

#[tokio::test]
#[serial]
async fn is_user_member_unknow_user_and_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = IsUserMemberQueryView::new(999, 999);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;
    assert!(
        result.is_ok(),
        "is_user_member_unknow_user_and_group failed: {:?}",
        result
    );
    assert!(
        !result.unwrap(),
        "is_user_member_unknow_user_and_group failed: result is true"
    );
}
