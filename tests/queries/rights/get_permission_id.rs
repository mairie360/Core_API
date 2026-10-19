use crate::common::get_pool;
use core_api::database::{
    ressources::get_ressource_type_id::GetRessourceTypeIdQueryView,
    rights::get_permission_id::{GetPermissionIdQueryView, PermissionAction},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn good_id_and_action() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("users");
    let ressource_type_id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = GetPermissionIdQueryView::new(ressource_type_id as u64, PermissionAction::ReadAll);
    println!("{view}");
    assert!(view.resource_id() > 0);
    assert_eq!(view.action(), PermissionAction::ReadAll);
    println!("{}", PermissionAction::Create);
    println!("{}", PermissionAction::Update);
    println!("{}", PermissionAction::Delete);
    println!("{}", PermissionAction::UpdateAll);
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();
    assert_eq!(result, 1, "Expected 1, got {result}");
}

#[tokio::test]
#[serial]
async fn fail_invalid_resource_id() {
    let pool = get_pool().await;
    let view = GetPermissionIdQueryView::new(100, PermissionAction::Read);
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn fail_invalid_action() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("users");
    let ressource_type_id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = GetPermissionIdQueryView::new(ressource_type_id as u64, PermissionAction::DeleteAll);
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_err());
}

#[tokio::test]
#[serial]
async fn fail_error_action() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let ressource_type_id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = GetPermissionIdQueryView::new(ressource_type_id as u64, PermissionAction::Error);
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_err());
}
