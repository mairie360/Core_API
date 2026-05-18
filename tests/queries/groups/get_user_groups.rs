use crate::common::get_pool;
use core_api::database::groups::get_user_groups::{get_user_groups, GetUserGroupsQuerView};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, GROUP_OWNER_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_groups_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserGroupsQuerView::new(*GROUP_OWNER_ID.get().unwrap() as u64);

    let result = get_user_groups(view, pool).await;
    assert!(result.clone().is_ok(), "{:?}", result.clone());
    assert!(!result.clone().unwrap().is_empty(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn get_user_groups_without_groups() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserGroupsQuerView::new(3);
    let result = get_user_groups(view, pool).await;
    assert!(result.clone().is_ok(), "{:?}", result.clone());
    assert!(result.clone().unwrap().is_empty(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn get_groups_bad_user_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserGroupsQuerView::new(999);
    let result = get_user_groups(view, pool).await;
    assert!(result.clone().is_ok(), "{:?}", result.clone());
    assert!(result.clone().unwrap().is_empty(), "{:?}", result);
}

// #[tokio::test]
// #[serial]
// async fn get_groups_bad_user_id() {
//     let (_container, host) = get_shared_db().await;
//     let pool = get_pool(host.to_string()).await;

//     let view = GetUserGroupsQuerView::new(999);
//     let result = get_user_groups(view, pool).await;
//     assert!(result.is_err(), "{:?}", result);
// }
