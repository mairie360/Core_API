use crate::common::get_pool;
use crate::common::users::{create_user, unique_marker};
use core_api::database::admin::list_users::{
    AdminCountUsersQueryView, AdminListUsersQueryView, AdminUserRow,
};
use core_api::database::groups::{
    add_user_to_group::AddUserToGroupQueryView, create_group::CreateGroupQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::ALICE_ID;

#[tokio::test]
async fn admin_list_users_searches_and_paginates_by_name() {
    let pool = get_pool().await;
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
async fn admin_list_users_includes_roles_and_matches_full_name() {
    let pool = get_pool().await;

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
async fn admin_list_users_filters_by_group() {
    let pool = get_pool().await;
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

/// Without a search the ids of the page come from the name order alone (MAIR-477): consecutive
/// pages follow each other exactly, the count covers every user, and the group filter still
/// applies.
#[tokio::test]
async fn admin_list_users_without_search_pages_every_user_in_name_order() {
    let host = crate::common::own_db_url().await;
    let pool = get_pool().await;
    let raw = crate::common::get_raw_pool(host.clone()).await;
    let marker = unique_marker("Unsearched");
    let member = create_user(&pool, "Member", &marker).await;
    let group_id: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(
            *ALICE_ID.get().unwrap() as u64,
            &marker,
            "admin_list_users_without_search",
        ))
        .await
        .unwrap();
    pool.execute(AddUserToGroupQueryView::new(group_id as u64, member as u64))
        .await
        .unwrap();

    let ids = |rows: &[AdminUserRow]| rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let page = |number, size| AdminListUsersQueryView::new(None, None, number, size);
    let first: Vec<AdminUserRow> = pool.fetch_all(&page(1, 3)).await.unwrap();
    let second: Vec<AdminUserRow> = pool.fetch_all(&page(2, 3)).await.unwrap();
    let both: Vec<AdminUserRow> = pool.fetch_all(&page(1, 6)).await.unwrap();
    let blank: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::new(Some("  "), None, 1, 6))
        .await
        .unwrap();
    let expected: Vec<i32> =
        sqlx::query_scalar("SELECT id FROM users ORDER BY last_name, first_name, id LIMIT 6")
            .fetch_all(&raw)
            .await
            .unwrap();
    let total: i64 = pool
        .fetch_scalar(&AdminCountUsersQueryView::new(None, None))
        .await
        .unwrap();
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&raw)
        .await
        .unwrap();
    let in_group: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::new(
            None,
            Some(group_id as u64),
            1,
            50,
        ))
        .await
        .unwrap();

    assert_eq!(ids(&both), expected);
    assert_eq!([ids(&first), ids(&second)].concat(), expected);
    // A blank search is no search.
    assert_eq!(ids(&blank), expected);
    assert_eq!(total, users);
    assert!(ids(&in_group).contains(&member));
    assert!(in_group
        .iter()
        .all(|row| row.id == member || row.id == *ALICE_ID.get().unwrap()));
}

/// MAIR-904: the list filters on the state shown by the administration and on the role, and sorts
/// by name both ways; the count follows the same filters.
#[tokio::test]
async fn mair_904_admin_list_users_filters_by_account_status_and_role_and_sorts_descending() {
    use core_api::database::admin::list_users::AdminUsersFilter;
    let pool = get_pool().await;
    let raw = crate::common::get_raw_pool(crate::common::own_db_url().await).await;
    let marker = unique_marker("Filter");
    let active = create_user(&pool, "Anna", &marker).await;
    let invited = create_user(&pool, "Ines", &marker).await;
    let suspended = create_user(&pool, "Sam", &marker).await;
    let archived = create_user(&pool, "Zoe", &marker).await;
    sqlx::query("UPDATE users SET first_connect = (id = $2), status = CASE WHEN id = $3 THEN 'suspended' ELSE status END WHERE id = ANY($1)")
        .bind(vec![active, invited, suspended, archived])
        .bind(invited)
        .bind(suspended)
        .execute(&raw)
        .await
        .unwrap();
    crate::common::users::archive_user(&raw, archived).await;
    sqlx::query("UPDATE user_roles SET role_id = (SELECT id FROM roles WHERE name = 'Admin') WHERE user_id = $1")
        .bind(active)
        .execute(&raw)
        .await
        .unwrap();
    let admin_role: i32 = sqlx::query_scalar("SELECT id FROM roles WHERE name = 'Admin'")
        .fetch_one(&raw)
        .await
        .unwrap();

    let ids = |rows: Vec<AdminUserRow>| {
        rows.into_iter()
            .map(|row| (row.id, row.account_status))
            .collect::<Vec<_>>()
    };
    let filter = |status: Option<&str>| AdminUsersFilter {
        search: Some(marker.clone()),
        account_status: status.map(str::to_string),
        ..AdminUsersFilter::default()
    };
    let all: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::filtered(
            &filter(None),
            false,
            1,
            10,
        ))
        .await
        .unwrap();
    assert_eq!(
        ids(all),
        vec![
            (active, "active".to_string()),
            (invited, "invited".to_string()),
            (suspended, "suspended".to_string()),
            (archived, "inactive".to_string()),
        ]
    );
    for (status, expected) in [
        ("active", active),
        ("invited", invited),
        ("suspended", suspended),
        ("inactive", archived),
    ] {
        let rows: Vec<AdminUserRow> = pool
            .fetch_all(&AdminListUsersQueryView::filtered(
                &filter(Some(status)),
                false,
                1,
                10,
            ))
            .await
            .unwrap();
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![expected],
            "{status}"
        );
        let count: i64 = pool
            .fetch_scalar(&AdminCountUsersQueryView::filtered(&filter(Some(status))))
            .await
            .unwrap();
        assert_eq!(count, 1, "{status}");
    }

    let by_role = AdminUsersFilter {
        role_id: Some(u64::try_from(admin_role).unwrap()),
        ..filter(None)
    };
    let rows: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::filtered(&by_role, false, 1, 10))
        .await
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![active]
    );

    let descending: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::filtered(
            &filter(None),
            true,
            1,
            2,
        ))
        .await
        .unwrap();
    assert_eq!(
        descending.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![archived, suspended]
    );

    // Without a search, the filters and the descending order hold too (index walk branch).
    let unsearched = AdminUsersFilter {
        account_status: Some("suspended".to_string()),
        ..AdminUsersFilter::default()
    };
    let rows: Vec<AdminUserRow> = pool
        .fetch_all(&AdminListUsersQueryView::filtered(&unsearched, true, 1, 50))
        .await
        .unwrap();
    assert!(rows.iter().any(|row| row.id == suspended));
    assert!(rows.iter().all(|row| row.account_status == "suspended"));
}
