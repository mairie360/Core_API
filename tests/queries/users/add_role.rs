use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::users::add_role::AddRolesQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_add_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = AddRolesQueryView::new(1, 2);
    println!("{}", view);
    assert_eq!(view.role_id(), 1);
    assert_eq!(view.user_id(), 2);

    let result = pool.execute(view).await;

    assert!(result.is_ok(), "execute should succeed, {:#?}", result);
}

#[tokio::test]
#[serial]
async fn test_add_role_bad_role_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = AddRolesQueryView::new(999, 1);

    let result = pool.execute(view).await;

    assert!(
        result.is_err(),
        "execute should fail with bad role_id, {:#?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn test_add_role_bad_user_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = AddRolesQueryView::new(1, 999);

    let result = pool.execute(view).await;

    assert!(
        result.is_err(),
        "execute should fail with bad user_id, {:#?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn test_add_role_bad_user_id_and_role_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = AddRolesQueryView::new(999, 999);

    let result = pool.execute(view).await;

    assert!(
        result.is_err(),
        "execute should fail with bad user_id and role_id, {:#?}",
        result
    );
}
