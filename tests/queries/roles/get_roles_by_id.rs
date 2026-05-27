use core_api::database::roles::get_roles_by_id::{get_roles_by_id_query, GetRolesByIdQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

use crate::common::{get_pool, roles::setup_tests};

#[tokio::test]
#[serial]
async fn test_get_roles_by_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let roles = get_roles_by_id_query(GetRolesByIdQueryView::new(vec![1, 2]), pool).await;

    assert!(
        roles.is_ok(),
        "roles should be retrieved successfully, got: {:#?}",
        roles
    );
    let roles = roles.unwrap();
    assert!(roles.len() == 2, "expected 2 roles, got: {}", roles.len());
}

#[tokio::test]
#[serial]
async fn test_get_roles_by_id_bad_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let roles = get_roles_by_id_query(GetRolesByIdQueryView::new(vec![998, 999]), pool).await;

    assert!(
        roles.is_ok(),
        "roles should be retrieved successfully, got: {:#?}",
        roles
    );
    let roles = roles.unwrap();
    assert!(roles.len() == 0, "expected 0 roles, got: {}", roles.len());
}
