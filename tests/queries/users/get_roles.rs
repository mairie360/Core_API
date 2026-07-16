use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::users::get_roles::{get_user_roles_query, GetUserRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_get_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserRolesQueryView::new(1);

    let result = get_user_roles_query(view, pool).await;

    assert!(
        result.is_ok(),
        "get_user_roles_query should succeed, {:#?}",
        result
    );
    let roles = result.unwrap();
    assert!(
        roles.len() >= 1,
        "get_user_roles_query should return at least one role, {:#?}",
        roles
    );
}

#[tokio::test]
#[serial]
async fn test_get_role_bad_user_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserRolesQueryView::new(999);

    let result = get_user_roles_query(view, pool).await;

    assert!(
        result.is_ok(),
        "get_user_roles_query should succeed, {:#?}",
        result
    );
    let roles = result.unwrap();
    assert!(
        roles.is_empty(),
        "get_user_roles_query should return an empty list for a non-existent role, {:#?}",
        roles
    );
}
