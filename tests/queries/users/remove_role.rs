use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::users::remove_role::{remove_role_query, RemoveRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_remove_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = RemoveRolesQueryView::new(1, 2);

    let result = remove_role_query(view, pool).await;

    assert!(
        result.is_ok(),
        "remove_role_query should succeed with valid role_id and user_id, {:?}",
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

    let result = remove_role_query(view, pool).await;

    assert!(
        result.is_ok(),
        "remove_role_query should succeed with bad role_id, {:?}",
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

    let result = remove_role_query(view, pool).await;

    assert!(
        result.is_ok(),
        "remove_role_query should succeed with bad user_id, {:?}",
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

    let result = remove_role_query(view, pool).await;

    assert!(
        result.is_ok(),
        "remove_role_query should succeed with bad user_id and role_id, {:?}",
        result
    );
}
