use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::users::remove_role::RemoveRolesQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_remove_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = RemoveRolesQueryView::new(1, 2);
    println!("{}", view);
    assert_eq!(view.role_id(), 1);
    assert_eq!(view.user_id(), 2);

    let result = pool.execute(view).await;

    assert!(
        result.is_ok(),
        "execute should succeed with valid role_id and user_id, {:?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn test_remove_role_bad_role_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = RemoveRolesQueryView::new(999, 1);

    let result = pool.execute(view).await;

    assert!(
        result.is_ok(),
        "execute should succeed with bad role_id, {:?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn test_remove_role_bad_user_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = RemoveRolesQueryView::new(1, 999);

    let result = pool.execute(view).await;

    assert!(
        result.is_ok(),
        "execute should succeed with bad user_id, {:?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn test_remove_role_bad_user_id_and_role_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = RemoveRolesQueryView::new(999, 999);

    let result = pool.execute(view).await;

    assert!(
        result.is_ok(),
        "execute should succeed with bad user_id and role_id, {:?}",
        result
    );
}
