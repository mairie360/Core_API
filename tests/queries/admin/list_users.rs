use crate::common::get_pool;
use crate::common::users::{create_user, unique_marker};
use core_api::database::admin::list_users::{
    AdminCountUsersQueryView, AdminListUsersQueryView, AdminUserRow,
};
use core_api::database::groups::{
    add_user_to_group::AddUserToGroupQueryView, create_group::CreateGroupQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn admin_list_users_searches_and_paginates_by_name() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let marker = unique_marker("Paging");
    let charlie = create_user(&pool, "Charlie", &marker).await;
    let alex = create_user(&pool, "Alex", &marker).await;
    let bea = create_user(&pool, "Bea", &marker).await;

    let first_page: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::new(Some(&marker), None, 1, 2))
        .await
        .unwrap();
    let second_page: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::new(Some(&marker), None, 2, 2))
        .await
        .unwrap();
    let total: i64 = pool
        .fetch_scalar(&AdminCountUsersQueryView::new(Some(&marker), None))
        .await
        .unwrap();

    // Même nom de famille : tri par prénom.
    let ids = |rows: &[AdminUserRow]| rows.iter().map(|row| row.id).collect::<Vec<_>>();
    assert_eq!(ids(&first_page), vec![alex, bea]);
    assert_eq!(ids(&second_page), vec![charlie]);
    assert_eq!(total, 3);
    assert!(!first_page[0].is_archived);
    // Sans rôle explicite, la base attribue Guest par défaut (Database >= 1.2.0).
    let role_names = first_page[0]
        .roles
        .iter()
        .map(|role| role.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(role_names, vec!["Guest"]);
}

#[tokio::test]
#[serial]
async fn admin_list_users_includes_roles_and_matches_full_name() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let rows: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::new(
            Some("  smith alice "),
            None,
            1,
            20,
        ))
        .await
        .unwrap();

    let alice = rows
        .iter()
        .find(|row| row.id == *ALICE_ID.get().unwrap())
        .expect("Alice trouvée par « nom prénom »");
    assert_eq!(alice.email, "alice@example.com");
    assert!(!alice.roles.is_empty());
}

#[tokio::test]
#[serial]
async fn admin_list_users_filters_by_group() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let marker = unique_marker("Grouped");
    let member = create_user(&pool, "Member", &marker).await;
    let _outsider = create_user(&pool, "Outsider", &marker).await;
    let group_id: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(
            *ALICE_ID.get().unwrap() as u64,
            &marker,
            "admin_list_users_filters_by_group",
        ))
        .await
        .unwrap();
    pool.execute(AddUserToGroupQueryView::new(group_id as u64, member as u64))
        .await
        .unwrap();

    let rows: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::new(
            Some(&marker),
            Some(group_id as u64),
            1,
            20,
        ))
        .await
        .unwrap();
    let total: i64 = pool
        .fetch_scalar(&AdminCountUsersQueryView::new(None, Some(group_id as u64)))
        .await
        .unwrap();

    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![member]
    );
    // Le créateur du groupe peut en être membre (trigger d'ajout du propriétaire).
    assert!(total >= 1);
}
