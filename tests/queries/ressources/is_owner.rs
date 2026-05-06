use crate::common::get_pool;
use core_api::database::ressources::is_owner::{is_owner_query, IsOwnerQueryView};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, GROUP_OWNER_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn true_result() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = IsOwnerQueryView::new(*GROUP_OWNER_ID.get().unwrap() as u64, 1, "groups");
    assert!(is_owner_query(view, pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn false_bad_ressource_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = IsOwnerQueryView::new(*GROUP_OWNER_ID.get().unwrap() as u64, 2, "groups");
    assert!(!is_owner_query(view, pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn false_bad_owner_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = IsOwnerQueryView::new((*GROUP_OWNER_ID.get().unwrap() as u64) + 1, 1, "groups");
    assert!(!is_owner_query(view, pool).await.unwrap());
}
