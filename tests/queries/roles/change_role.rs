use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::roles::change_role::{change_role_query, ChangeRoleQueryView};
use core_api::database::roles::create_role::{create_role_query, CreateRoleQueryView};
use core_api::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_change_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CreateRoleQueryView::new("Change_test", "Change_test role", Some(true));
    let result = create_role_query(view, pool.clone()).await;

    assert!(result.is_ok());

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

    let mut new_role_id: i32 = 0;
    for role in roles {
        if role.name() == "Change_test" {
            new_role_id = role.id();
            break;
        }
    }

    let view = ChangeRoleQueryView::new(
        new_role_id as u64,
        "Change_Admin",
        "Change_Administrateur",
        Some(true),
    );
    let result = change_role_query(view, pool.clone()).await;

    assert!(result.is_ok());

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

    for role in roles {
        if role
            .description()
            .is_some_and(|d| d == "Change_Administrateur".to_string())
        {
            assert_eq!(role.id(), new_role_id);
            assert_eq!(role.name(), "Change_Admin");
            assert_eq!(role.description().unwrap(), "Change_Administrateur");
            assert!(role.updated_at().is_some());
            assert!(role.can_be_deleted());
        }
    }
}

#[tokio::test]
#[serial]
async fn test_change_role_bad_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = ChangeRoleQueryView::new(999, "Admin", "Administrateur", Some(false));
    let result = change_role_query(view, pool.clone()).await;

    assert!(result.is_err());
}
