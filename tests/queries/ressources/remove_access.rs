use crate::common::get_pool;
use core_api::database::{
    ressources::{
        add_access_to_user::AddAccessToUserQueryView,
        get_ressource_type_id::GetRessourceTypeIdQueryView, remove_access::RemoveAccessQueryView,
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
    let id: i32 = pool.fetch_scalar(&view).await.unwrap();
    let id = id as u64;
    let view = GetPermissionIdQueryView::new(id, PermissionAction::Read);
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();
    let result = result as u64;
    let view = AddAccessToUserQueryView::new(3, id, 1, result);
    let _ = pool.execute(view).await;
    let view = RemoveAccessQueryView::new(2);
    println!("{}", view);
    assert_eq!(view.id(), 2);
    let result = pool.execute(view).await;
    assert!(result.is_ok(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn bad_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = RemoveAccessQueryView::new(3);
    let result = pool.execute(view).await;
    assert!(result.is_ok(), "{:?}", result);
}
