use crate::common::get_pool;
use core_api::database::{
    ressources::get_ressource_type_id::{get_ressource_type_id_query, GetRessourceTypeIdQueryView},
    rights::get_permission_id::{
        get_permission_id_query, GetPermissionIdQueryView, PermissionAction,
    },
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn good_id_and_action() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("users");
    let view = GetPermissionIdQueryView::new(
        get_ressource_type_id_query(view, pool.clone())
            .await
            .unwrap(),
        PermissionAction::ReadAll,
    );
    let result = get_permission_id_query(view, pool).await.unwrap();
    assert_eq!(result, 1, "{}", format!("Expected 1, got {}", result));
}

#[tokio::test]
#[serial]
async fn fail_invalid_resource_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetPermissionIdQueryView::new(100, PermissionAction::Read);
    assert!(get_permission_id_query(view, pool).await.is_err());
}

#[tokio::test]
#[serial]
async fn fail_invalid_action() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("users");
    let view = GetPermissionIdQueryView::new(
        get_ressource_type_id_query(view, pool.clone())
            .await
            .unwrap(),
        PermissionAction::DeleteAll,
    );
    assert!(get_permission_id_query(view, pool).await.is_err());
}

#[tokio::test]
#[serial]
async fn fail_error_action() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let view = GetPermissionIdQueryView::new(
        get_ressource_type_id_query(view, pool.clone())
            .await
            .unwrap(),
        PermissionAction::Error,
    );
    assert!(get_permission_id_query(view, pool).await.is_err());
}
