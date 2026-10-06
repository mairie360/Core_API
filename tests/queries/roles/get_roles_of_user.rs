use core_api::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use core_api::database::roles::get_roles_of_user::GetRolesOfUserQueryView;
use core_api::database::users::get_roles::GetUserRolesQueryView;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID};
use serial_test::serial;

use crate::common::{get_pool, roles::setup_tests};

/// Same rows, in the same order, as the two round trips it replaces (MAIR-474).
#[tokio::test]
#[serial]
async fn test_get_roles_of_user_matches_ids_then_roles() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let admin = u64::try_from(*ADMIN_ID.get().unwrap()).unwrap();

    let ids: Vec<i32> = pool
        .fetch_all(&GetUserRolesQueryView::new(admin))
        .await
        .unwrap();
    let expected: Vec<Role> = pool
        .fetch_all(&GetRolesByIdQueryView::new(ids))
        .await
        .unwrap();
    let roles: Vec<Role> = pool
        .fetch_all(&GetRolesOfUserQueryView::new(admin))
        .await
        .unwrap();

    assert!(!roles.is_empty(), "the admin holds at least one role");
    assert_eq!(roles, expected);
}

#[tokio::test]
#[serial]
async fn test_get_roles_of_unknown_user_is_empty() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = GetRolesOfUserQueryView::new(999_999);
    println!("{view}");
    let roles: Vec<Role> = pool.fetch_all(&view).await.unwrap();

    assert!(roles.is_empty(), "expected 0 roles, got: {}", roles.len());
}
