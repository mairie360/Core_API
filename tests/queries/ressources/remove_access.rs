use crate::common::get_pool;
use core_api::database::{
    ressources::{
        add_access_to_user::{add_access_to_user_query, AddAccessToUserQueryView},
        get_ressource_type_id::{get_ressource_type_id_query, GetRessourceTypeIdQueryView},
        remove_access::{remove_access_query, RemoveAccessQueryView},
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
    let view = AddAccessToUserQueryView::new(3, id, 1, result);
    let _ = add_access_to_user_query(view, pool.clone()).await;
    let view = RemoveAccessQueryView::new(2);
    let result = remove_access_query(view, pool).await;
    assert!(result.is_ok(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn bad_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = RemoveAccessQueryView::new(3);
    let result = remove_access_query(view, pool).await;
    assert!(result.is_ok(), "{:?}", result);
}
