use core_api::database::roles::get_roles_by_id::{GetRolesByIdQueryView, Role};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

use crate::common::{get_pool, roles::setup_tests};

#[tokio::test]
#[serial]
async fn test_get_roles_by_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetRolesByIdQueryView::new(vec![1, 2]);
    println!("{}", view);
    let roles: Result<Vec<Role>, _> = pool.fetch_all(&view).await;

    assert!(
        roles.is_ok(),
        "roles should be retrieved successfully, got: {:#?}",
        roles
    );
    let roles = roles.unwrap();
    assert!(roles.len() == 2, "expected 2 roles, got: {}", roles.len());

    let role = &roles[0];
    println!("{}", role);
    assert!(!role.name().is_empty());
    let _ = role.description();
    let _ = role.updated_at();
    let _ = role.can_be_deleted();
    let _ = role.created_at();
}

#[tokio::test]
#[serial]
async fn test_get_roles_by_id_bad_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let roles: Result<Vec<Role>, _> = pool
        .fetch_all(&GetRolesByIdQueryView::new(vec![998, 999]))
        .await;

    assert!(
        roles.is_ok(),
        "roles should be retrieved successfully, got: {:#?}",
        roles
    );
    let roles = roles.unwrap();
    assert!(roles.is_empty(), "expected 0 roles, got: {}", roles.len());
}
