use crate::common::get_pool;
use core_api::database::groups::create_group::CreateGroupQueryView;
use core_api::database::groups::get_group::Group;
use core_api::database::groups::update_group::UpdateGroupQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn update_group_keeps_fields_that_are_not_provided() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let owner = *ALICE_ID.get().unwrap();
    let group_id: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(
            owner as u64,
            "update_group_name",
            "update_group_description",
        ))
        .await
        .unwrap();

    let renamed: Vec<Group> = pool
        .fetch_all(&UpdateGroupQueryView::new(
            group_id as u64,
            Some("update_group_renamed"),
            None,
        ))
        .await
        .unwrap();
    let described: Vec<Group> = pool
        .fetch_all(&UpdateGroupQueryView::new(
            group_id as u64,
            None,
            Some("nouvelle description"),
        ))
        .await
        .unwrap();

    assert_eq!(
        renamed,
        vec![Group::new(
            group_id,
            "update_group_renamed",
            owner,
            Some("update_group_description")
        )]
    );
    assert_eq!(
        described,
        vec![Group::new(
            group_id,
            "update_group_renamed",
            owner,
            Some("nouvelle description")
        )]
    );
}

#[tokio::test]
#[serial]
async fn update_group_unknown_group_returns_no_row() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: Vec<Group> = pool
        .fetch_all(&UpdateGroupQueryView::new(999_999, Some("inconnu"), None))
        .await
        .unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
#[serial]
async fn update_group_to_a_taken_name_is_a_unique_violation() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let owner = *ALICE_ID.get().unwrap() as u64;
    let _: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(owner, "update_group_taken", ""))
        .await
        .unwrap();
    let group_id: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(owner, "update_group_other", ""))
        .await
        .unwrap();

    // The endpoint maps this unique violation to 409.
    let result: Result<Vec<Group>, _> = pool
        .fetch_all(&UpdateGroupQueryView::new(
            group_id as u64,
            Some("update_group_taken"),
            None,
        ))
        .await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::UniqueViolation(_)))
    ));
}
