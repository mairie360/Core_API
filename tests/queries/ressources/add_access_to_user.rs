use crate::common::get_pool;
use core_api::database::ressources::{
    add_access_to_user::{add_access_to_user_query, AddAccessToUserQueryView},
    get_ressource_type_id::{get_ressource_type_id_query, GetRessourceTypeIdQueryView},
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
    let view = AddAccessToUserQueryView::new(2, id, 1, 23);
    assert!(add_access_to_user_query(view, pool).await.is_ok());
}

#[tokio::test]
#[serial]
async fn failure_add_all_right() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id = get_ressource_type_id_query(view, pool.clone())
        .await
        .unwrap();
    let view = AddAccessToUserQueryView::new(2, id, 1, 1);
    assert!(add_access_to_user_query(view, pool).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_target_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id = get_ressource_type_id_query(view, pool.clone())
        .await
        .unwrap();
    let view = AddAccessToUserQueryView::new(10, id, 1, 1);
    assert!(add_access_to_user_query(view, pool).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_ressource_type_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = AddAccessToUserQueryView::new(10, 100, 1, 1);
    assert!(add_access_to_user_query(view, pool).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_ressource_instance_type_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id = get_ressource_type_id_query(view, pool.clone())
        .await
        .unwrap();
    let view = AddAccessToUserQueryView::new(10, id, 100, 1);
    assert!(add_access_to_user_query(view, pool).await.is_err());
}

#[tokio::test]
#[serial]
async fn failure_bad_access_type_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("groups");
    let id = get_ressource_type_id_query(view, pool.clone())
        .await
        .unwrap();
    let view = AddAccessToUserQueryView::new(10, id, 1, 100);
    assert!(add_access_to_user_query(view, pool).await.is_err());
}
