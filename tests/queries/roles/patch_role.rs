use crate::common::{
    get_pool,
    roles::{PATCH_ID, PATCH_MUTEX},
};
use core_api::database::roles::{
    get_roles::{GetRolesQueryView, RoleQueryResult},
    patch_role::PatchRoleQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

async fn patch_role(
    pool: &mairie360_api_lib::smart_db::SmartDatabase,
    view: PatchRoleQueryView,
) -> Result<(), mairie360_api_lib::error::ApiLibError> {
    if view.is_noop() {
        return Ok(());
    }
    pool.execute(view).await
}

#[tokio::test]
#[serial]
async fn test_patch_role_name() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = PatchRoleQueryView::new(
        *PATCH_ID.get().unwrap(),
        Some("Patch".to_string()),
        None,
        None,
    );
    println!("{}", view);
    println!("{:?}", view);
    assert_eq!(view.id(), *PATCH_ID.get().unwrap());
    assert_eq!(view.name(), Some("Patch"));
    assert_eq!(view.description(), None);
    assert_eq!(view.can_be_deleted(), None);
    assert!(!view.is_noop());
    let _guard = PATCH_MUTEX.get().unwrap().lock().await;
    let result = patch_role(&pool, view).await;

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

    assert!(result.is_ok());
    for role in roles {
        if role.id() == *PATCH_ID.get().unwrap() as i32 {
            assert_eq!(role.name(), "Patch");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_patch_role_description() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = PatchRoleQueryView::new(
        *PATCH_ID.get().unwrap(),
        None,
        Some("Patch description".to_string()),
        None,
    );
    let _guard = PATCH_MUTEX.get().unwrap().lock().await;
    let result = patch_role(&pool, view).await;

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

    assert!(result.is_ok());
    for role in roles {
        if role.id() == *PATCH_ID.get().unwrap() as i32 {
            assert_eq!(role.description().unwrap(), "Patch description");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_patch_role_can_be_deleted_to_false() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = PatchRoleQueryView::new(*PATCH_ID.get().unwrap(), None, None, Some(Some(false)));
    let _guard = PATCH_MUTEX.get().unwrap().lock().await;
    let result = patch_role(&pool, view).await;

    let roles: Vec<RoleQueryResult> = pool.fetch_all(&GetRolesQueryView::default()).await.unwrap();

    assert!(result.is_ok());
    for role in roles {
        if role.id() == *PATCH_ID.get().unwrap() as i32 {
            assert!(!role.can_be_deleted());
        }
    }
}
