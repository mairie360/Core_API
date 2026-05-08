use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::roles::change_role::{change_role_query, ChangeRoleQueryView};
use core_api::database::roles::create_role::{create_role_query, CreateRoleQueryView};
use core_api::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use rand::random;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_change_role_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let name = "test_change_role_success".to_string() + random::<u64>().to_string().as_str();
    let description =
        "test_change_role_success_description".to_string() + random::<u64>().to_string().as_str();

    let view = CreateRoleQueryView::new(&name, &description, Some(true));
    let result = create_role_query(view, pool.clone()).await;

    assert!(result.is_ok());

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

    let mut new_role_id: i32 = 0;
    for role in roles {
        if role.name() == name {
            new_role_id = role.id();
            break;
        }
    }

    let change_name = "Change_Admin".to_string() + random::<u64>().to_string().as_str();
    let change_description =
        "Change_Administrateur".to_string() + random::<u64>().to_string().as_str();

    let view = ChangeRoleQueryView::new(
        new_role_id as u64,
        &change_name,
        &change_description,
        Some(true),
    );
    let result = change_role_query(view, pool.clone()).await;

    assert!(result.is_ok());

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

    for role in roles {
        if role.description().is_some_and(|d| d == change_description) {
            assert_eq!(role.id(), new_role_id);
            assert_eq!(role.name(), change_name);
            assert_eq!(role.description().unwrap(), change_description);
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
