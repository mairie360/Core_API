use crate::common::get_pool;
use core_api::database::{
    ressources::{
        add_access_to_user::AddAccessToUserQueryView,
        get_access_by_ressource::GetAccessByRessourceQueryView,
        get_ressource_type_id::GetRessourceTypeIdQueryView,
    },
    rights::get_permission_id::{GetPermissionIdQueryView, PermissionAction},
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let ressource_type_id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let ressource_type_id = ressource_type_id as u64;
    let view = GetPermissionIdQueryView::new(ressource_type_id, PermissionAction::Read);
    let permission_id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let permission_id = permission_id as u64;
    let view = AddAccessToUserQueryView::new(4, ressource_type_id, 1, permission_id);
    let _ = pool.execute(view).await;
    let view = GetAccessByRessourceQueryView::new(1);
    println!("{}", view);
    let result: Vec<core_api::database::ressources::get_access_by_ressource::Access> =
        pool.fetch_all(&view).await.unwrap();
    assert!(!result.is_empty(), "{:?}", result);

    // La table `access_control` est partagée avec d'autres tests (même conteneur, même
    // `resource_instance_id`), donc on cherche la ligne qu'on vient d'insérer plutôt que de
    // supposer un index fixe.
    let access = result
        .iter()
        .find(|access| {
            access.user_id() == Some(4) && access.resource_id() == ressource_type_id as i32
        })
        .expect("just-inserted access row should be present");
    println!("{}", access);
    assert!(access.id() > 0);
    assert_eq!(access.group_id(), None);
    assert_eq!(access.resource_instance_id(), 1);
    assert_eq!(access.permission_id(), permission_id as i32);
}

#[tokio::test]
#[serial]
async fn unknow_ressource() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetAccessByRessourceQueryView::new(2);
    let result: Vec<core_api::database::ressources::get_access_by_ressource::Access> =
        pool.fetch_all(&view).await.unwrap();
    assert!(result.is_empty(), "{:?}", result);
}
