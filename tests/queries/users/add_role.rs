use crate::common::get_pool;
use crate::common::roles::setup_tests;
use crate::common::users::{create_user, unique_marker};
use core_api::database::users::add_role::AddRolesQueryView;
use core_api::database::users::get_roles::GetUserRolesQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_add_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = AddRolesQueryView::new(1, 2);
    println!("{view}");
    assert_eq!(view.role_id(), 1);
    assert_eq!(view.user_id(), 2);

    let result = pool.execute(view).await;

    assert!(result.is_ok(), "execute should succeed, {result:#?}");
}

#[tokio::test]
#[serial]
async fn mair_564_add_role_replaces_the_role_held() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let user = create_user(&pool, "Replace", &unique_marker("addrole")).await;
    let user_id = u64::try_from(user).unwrap();

    // Guest (given by the schema) replaced by role 2, then role 2 granted again.
    let granted: Vec<i32> = pool
        .fetch_all(&AddRolesQueryView::new(2, user_id))
        .await
        .unwrap();
    assert_eq!(granted, vec![2]);
    let again: Vec<i32> = pool
        .fetch_all(&AddRolesQueryView::new(2, user_id))
        .await
        .unwrap();
    assert!(again.is_empty(), "the role already held returns no row");

    let roles: Vec<i32> = pool
        .fetch_all(&GetUserRolesQueryView::new(user_id))
        .await
        .unwrap();
    assert_eq!(roles, vec![2]);
}

#[tokio::test]
#[serial]
async fn test_add_role_bad_role_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = AddRolesQueryView::new(999, 1);

    let result = pool.execute(view).await;

    assert!(
        result.is_err(),
        "execute should fail with bad role_id, {result:#?}"
    );
}

#[tokio::test]
#[serial]
async fn test_add_role_bad_user_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = AddRolesQueryView::new(1, 999);

    let result = pool.execute(view).await;

    assert!(
        result.is_err(),
        "execute should fail with bad user_id, {result:#?}"
    );
}

#[tokio::test]
#[serial]
async fn test_add_role_bad_user_id_and_role_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = AddRolesQueryView::new(999, 999);

    let result = pool.execute(view).await;

    assert!(
        result.is_err(),
        "execute should fail with bad user_id and role_id, {result:#?}"
    );
}
