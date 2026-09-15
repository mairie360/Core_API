use crate::common::users::{create_user, unique_marker};
use crate::common::{get_pool, get_raw_pool};
use core_api::database::groups::{
    add_user_to_group::AddUserToGroupQueryView, create_group::CreateGroupQueryView,
};
use core_api::database::users::list_directory::{DirectoryUser, ListDirectoryUsersQueryView};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

fn ids(users: &[DirectoryUser]) -> Vec<i32> {
    users.iter().map(|user| user.id).collect()
}

#[tokio::test]
#[serial]
async fn directory_searches_non_archived_users_sorted_by_name() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let raw = get_raw_pool(host.to_string()).await;
    let marker = unique_marker("Directory");
    let zoe = create_user(&pool, "Zoe", &marker).await;
    let adam = create_user(&pool, "Adam", &marker).await;
    let archived = create_user(&pool, "Archived", &marker).await;
    sqlx::query("UPDATE users SET is_archived = true WHERE id = $1")
        .bind(archived)
        .execute(&raw)
        .await
        .unwrap();

    let users: Vec<DirectoryUser> = pool
        .fetch_all(&ListDirectoryUsersQueryView::new(
            Some(&marker.to_lowercase()),
            &[],
            &[],
            1000,
        ))
        .await
        .unwrap();
    let limited: Vec<DirectoryUser> = pool
        .fetch_all(&ListDirectoryUsersQueryView::new(
            Some(&marker),
            &[],
            &[],
            1,
        ))
        .await
        .unwrap();

    assert_eq!(ids(&users), vec![adam, zoe]);
    assert_eq!(ids(&limited), vec![adam]);
    assert_eq!(users[0].first_name, "Adam");
    assert_eq!(users[0].last_name, marker);
    assert!(users[0].roles.is_empty());
    assert!(users[0].group_ids.is_empty());
}

#[tokio::test]
#[serial]
async fn directory_filters_by_ids_and_groups_and_exposes_roles() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let marker = unique_marker("Scoped");
    let member = create_user(&pool, "Member", &marker).await;
    let other = create_user(&pool, "Other", &marker).await;
    let alice = *ALICE_ID.get().unwrap();
    let group_id: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(
            alice as u64,
            &marker,
            "directory_filters_by_ids_and_groups",
        ))
        .await
        .unwrap();
    pool.execute(AddUserToGroupQueryView::new(group_id as u64, member as u64))
        .await
        .unwrap();

    let by_ids: Vec<DirectoryUser> = pool
        .fetch_all(&ListDirectoryUsersQueryView::new(
            None,
            &[other as u64, alice as u64],
            &[],
            1000,
        ))
        .await
        .unwrap();
    let by_group: Vec<DirectoryUser> = pool
        .fetch_all(&ListDirectoryUsersQueryView::new(
            Some(&marker),
            &[],
            &[group_id as u64, 999_999],
            1000,
        ))
        .await
        .unwrap();

    assert_eq!(by_ids.len(), 2);
    assert!(by_ids.iter().any(|user| user.id == other));
    let alice_entry = by_ids.iter().find(|user| user.id == alice).unwrap();
    assert!(!alice_entry.roles.is_empty());
    assert_eq!(ids(&by_group), vec![member]);
    assert_eq!(by_group[0].group_ids, vec![group_id]);
}
