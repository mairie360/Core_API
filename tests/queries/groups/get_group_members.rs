use crate::common::get_pool;
use core_api::database::groups::{
    get_group::Group, get_group_members::GetGroupUsersQueryView,
    get_user_groups::GetUserGroupsQuerView,
};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, GROUP_OWNER_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_group_members_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserGroupsQuerView::new(*GROUP_OWNER_ID.get().unwrap() as u64);
    println!("{}", view);
    let result: Vec<Group> = pool.fetch_all(&view).await.unwrap();
    println!("{}", result[0]);

    let view = GetGroupUsersQueryView::new(result[0].id() as u64);
    println!("{}", view);
    let result: Result<Vec<i32>, _> = pool.fetch_all(&view).await;

    assert!(result.is_ok(), "Result should be Ok, got {:?}", result);
    let result = result.unwrap();

    assert!(
        !result.is_empty(),
        "Result should not be empty, got {:?}",
        result
    );

    assert_eq!(
        result.len(),
        1,
        "Result should have 1 element, got {:?}",
        result
    );

    assert_eq!(
        result[0],
        *GROUP_OWNER_ID.get().unwrap(),
        "Result should have the owner user_id: {}, got {}",
        *GROUP_OWNER_ID.get().unwrap(),
        result[0]
    );
}

#[tokio::test]
#[serial]
async fn get_group_members_bad_group_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetGroupUsersQueryView::new(999);
    let result: Result<Vec<i32>, _> = pool.fetch_all(&view).await;

    assert!(result.is_ok(), "Result should be Ok, got {:?}", result);
    let result = result.unwrap();

    assert!(
        result.is_empty(),
        "Result should be empty, got {:?}",
        result
    );
}
