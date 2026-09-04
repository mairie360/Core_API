use crate::common::get_pool;
use core_api::database::roles::create_role::CreateRoleQueryView;
use core_api::database::roles::get_roles::{GetRolesQueryView, RoleQueryResult};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use rand::random;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_role_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let nb_roles: Vec<RoleQueryResult> =
        pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();
    let nb_roles = nb_roles.len();

    // Aucun `can_be_deleted` fourni : deuxième branche statique de `query_sql`.
    let no_flag_name =
        "create_role_no_flag_throwaway".to_string() + random::<u64>().to_string().as_str();
    let no_flag_view = CreateRoleQueryView::new(
        &no_flag_name,
        "create_role_no_flag_throwaway_description",
        None,
    );
    println!("{}", no_flag_view);
    assert_eq!(no_flag_view.can_be_deleted(), None);
    assert!(pool.execute(no_flag_view).await.is_ok());

    let name = "create_role_success".to_string() + random::<u64>().to_string().as_str();
    let description =
        "create_role_success_description".to_string() + random::<u64>().to_string().as_str();
    let view = CreateRoleQueryView::new(&name, &description, Some(false));
    println!("{}", view);
    assert_eq!(view.name(), name);
    assert_eq!(view.description(), description);
    assert_eq!(view.can_be_deleted(), Some(false));
    let result = pool.execute(view).await;

    assert!(result.is_ok());

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();
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
