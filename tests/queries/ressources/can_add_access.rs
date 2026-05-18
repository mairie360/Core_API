use crate::common::get_pool;
use core_api::database::ressources::can_add_access::{can_add_access_query, CanAddAccessQueryView};
use core_api::endpoints::v1::ressources::AccessType;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, GROUP_OWNER_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = CanAddAccessQueryView::new(
        *GROUP_OWNER_ID.get().unwrap() as u64,
        2,
        1,
        "groups",
        AccessType::Read,
    );
    assert!(can_add_access_query(view, pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn failure_bad_owner_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = CanAddAccessQueryView::new(
        (*GROUP_OWNER_ID.get().unwrap() as u64) + 1,
        2,
        1,
        "groups",
        AccessType::Read,
    );
    assert!(!can_add_access_query(view, pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn failure_bad_ressource_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = CanAddAccessQueryView::new(
        *GROUP_OWNER_ID.get().unwrap() as u64,
        2,
        2,
        "groups",
        AccessType::Read,
    );
    assert!(!can_add_access_query(view, pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn failure_access_type_error() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = CanAddAccessQueryView::new(
        *GROUP_OWNER_ID.get().unwrap() as u64,
        1,
        1,
        "groups",
        AccessType::Error,
    );
    assert!(!can_add_access_query(view, pool).await.unwrap());
}
