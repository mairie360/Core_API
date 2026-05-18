use crate::common::{
    get_pool,
    roles::{PATCH_ID, PATCH_MUTEX},
};
use core_api::database::roles::{
    get_roles::{get_roles_query, GetRolesQueryView},
    patch_role::{patch_role_query, PatchRoleQueryView},
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

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
    let _guard = PATCH_MUTEX.get().unwrap().lock().await;
    let result = patch_role_query(view, pool.clone()).await;

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

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
    let result = patch_role_query(view, pool.clone()).await;

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

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
    let result = patch_role_query(view, pool.clone()).await;

    let roles = get_roles_query(GetRolesQueryView {}, pool.clone())
        .await
        .unwrap();

    assert!(result.is_ok());
    for role in roles {
        if role.id() == *PATCH_ID.get().unwrap() as i32 {
            assert_eq!(role.can_be_deleted(), false);
        }
    }
}
