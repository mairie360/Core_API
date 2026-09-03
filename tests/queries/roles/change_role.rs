use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::roles::change_role::ChangeRoleQueryView;
use core_api::database::roles::create_role::CreateRoleQueryView;
use core_api::database::roles::does_role_exist::DoesRoleExistQueryView;
use core_api::database::roles::get_roles::{GetRolesQueryView, RoleQueryResult};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use rand::random;
use serial_test::serial;

// Reprend le pré-check fait par `admin/roles/put/endpoint.rs` avant d'exécuter la mise à jour.
async fn change_role(
    smart_db: &SmartDatabase,
    view: ChangeRoleQueryView,
) -> Result<(), ApiLibError> {
    let exists: bool = smart_db
        .fetch_scalar(&DoesRoleExistQueryView::new(view.id()))
        .await?;
    if !exists {
        return Err(ApiLibError::Database(DbError::NotFound));
    }
    smart_db.execute(view).await
}

#[tokio::test]
#[serial]
async fn test_change_role_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let name = "test_change_role_success".to_string() + random::<u64>().to_string().as_str();
    let description =
        "test_change_role_success_description".to_string() + random::<u64>().to_string().as_str();

    let view = CreateRoleQueryView::new(&name, &description, Some(true));
    let result = pool.execute(view).await;

    assert!(result.is_ok());

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

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
    println!("{}", view);
    assert_eq!(view.id(), new_role_id as u64);
    assert_eq!(view.name(), change_name);
    assert_eq!(view.description(), change_description);
    assert_eq!(view.can_be_deleted(), Some(true));
    let result = change_role(&pool, view).await;

    assert!(result.is_ok());

    // Pas de `can_be_deleted` fourni : deuxième branche statique de `query_sql`.
    let no_flag_view =
        ChangeRoleQueryView::new(new_role_id as u64, &change_name, &change_description, None);
    assert!(change_role(&pool, no_flag_view).await.is_ok());

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

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
    let result = change_role(&pool, view).await;

    assert!(result.is_err());
}
