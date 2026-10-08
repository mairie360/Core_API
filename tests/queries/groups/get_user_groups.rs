use crate::common::get_pool;
use crate::common::users::{create_user, unique_marker};
use core_api::database::groups::get_group::Group;
use core_api::database::groups::get_user_groups::GetUserGroupsQuerView;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, GROUP_OWNER_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_groups_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = GetUserGroupsQuerView::new(*GROUP_OWNER_ID.get().unwrap() as u64);
    println!("{view}");

    let result: Result<Vec<Group>, _> = pool.fetch_all(&view).await;
    assert!(result.is_ok(), "{result:?}");
    assert!(!result.unwrap().is_empty());
}

#[tokio::test]
#[serial]
async fn get_user_groups_without_groups() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    // A fresh account: a fixed id may belong to a group created by another test of the binary
    // (the shared database is seeded in test order).
    let user = create_user(&pool, "Lonely", &unique_marker("nogroup")).await;
    let view = GetUserGroupsQuerView::new(u64::try_from(user).unwrap());
    let result: Result<Vec<Group>, _> = pool.fetch_all(&view).await;
    assert!(result.is_ok(), "{result:?}");
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
#[serial]
async fn get_groups_bad_user_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = GetUserGroupsQuerView::new(999);
    let result: Result<Vec<Group>, _> = pool.fetch_all(&view).await;
    assert!(result.is_ok(), "{result:?}");
    assert!(result.unwrap().is_empty());
}
