use crate::common::get_pool;
use core_api::database::roles::create_role::{create_role_query, CreateRoleQueryView};
use core_api::database::roles::get_roles::{get_roles_query, GetRolesQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use rand::random;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_role_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let nb_roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap()
        .len();

    let name = "create_role_success".to_string() + random::<u64>().to_string().as_str();
    let description =
        "create_role_success_description".to_string() + random::<u64>().to_string().as_str();
    let view = CreateRoleQueryView::new(&name, &description, Some(false));
    let result = create_role_query(view, pool.clone()).await;

    assert!(result.is_ok());

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();
    assert!(roles.len() >= nb_roles);

    for role in roles {
        if role.description().is_some_and(|d| d == description) {
            assert!(role.id() > 2);
            assert_eq!(role.name(), name);
            assert_eq!(role.description().unwrap(), description);
            assert!(role.updated_at().is_some());
            assert!(!role.can_be_deleted());
        }
    }
}
