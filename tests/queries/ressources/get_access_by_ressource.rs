use crate::common::get_pool;
use core_api::database::{
    ressources::{
        add_access_to_user::{add_access_to_user_query, AddAccessToUserQueryView},
        get_access_by_ressource::{get_access_by_ressource, GetAccessByRessourceQueryView},
        get_ressource_type_id::{get_ressource_type_id_query, GetRessourceTypeIdQueryView},
    },
    rights::get_permission_id::{
        get_permission_id_query, GetPermissionIdQueryView, PermissionAction,
    },
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id = get_ressource_type_id_query(view, pool.clone())
        .await
        .unwrap();
    let view = GetPermissionIdQueryView::new(id, PermissionAction::Read);
    let result = get_permission_id_query(view, pool.clone()).await.unwrap();
    let view = AddAccessToUserQueryView::new(4, id, 1, result);
    let _ = add_access_to_user_query(view, pool.clone()).await;
    let view = GetAccessByRessourceQueryView::new(1);
    let result = get_access_by_ressource(view, pool).await.unwrap();
    assert!(!result.is_empty(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn unknow_ressource() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetAccessByRessourceQueryView::new(2);
    let result = get_access_by_ressource(view, pool).await.unwrap();
    assert!(result.is_empty(), "{:?}", result);
}
