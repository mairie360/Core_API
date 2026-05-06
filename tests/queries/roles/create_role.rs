use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::roles::create_role::{create_role_query, CreateRoleQueryView};
use core_api::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let nb_roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap()
        .len();

    let view = CreateRoleQueryView::new("Create", "Create role", Some(false));
    let result = create_role_query(view, pool.clone()).await;

    assert!(result.is_ok());

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();
    assert!(roles.len() >= nb_roles);

    for role in roles {
        if role.description().is_some_and(|d| d == "Create role") {
            assert!(role.id() > 2);
            assert_eq!(role.name(), "Create");
            assert_eq!(role.description().unwrap(), "Create role");
            assert!(role.updated_at().is_some());
            assert!(!role.can_be_deleted());
        }
    }
}
