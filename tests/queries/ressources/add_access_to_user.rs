use crate::common::get_pool;
use core_api::database::{
    ressources::{
        add_access_to_user::AddAccessToUserQueryView,
        get_ressource_type_id::GetRessourceTypeIdQueryView,
    },
    rights::get_permission_id::{GetPermissionIdQueryView, PermissionAction},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let id = id as u64;
    let view = GetPermissionIdQueryView::new(id, PermissionAction::Read);
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();
    let result = result as u64;
    let view = AddAccessToUserQueryView::new(2, id, 1, result);
    println!("{view}");
    assert_eq!(view.user_id(), 2);
    assert_eq!(view.ressource_type_id(), id);
    assert_eq!(view.ressource_instance_id(), 1);
    assert_eq!(view.access_type_id(), result);
    let result = pool.execute(view).await;
    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
#[serial]
async fn failure_add_all_right() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = AddAccessToUserQueryView::new(2, id as u64, 1, 1);
    assert!(pool.execute(view).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_target_id() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = AddAccessToUserQueryView::new(10, id as u64, 1, 1);
    assert!(pool.execute(view).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_ressource_type_id() {
    let pool = get_pool().await;
    let view = AddAccessToUserQueryView::new(10, 100, 1, 1);
    assert!(pool.execute(view).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_ressource_instance_type_id() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = AddAccessToUserQueryView::new(10, id as u64, 100, 1);
    assert!(pool.execute(view).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_access_type_id() {
    let pool = get_pool().await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let view = AddAccessToUserQueryView::new(10, id as u64, 1, 100);
    assert!(pool.execute(view).await.is_err());
}
