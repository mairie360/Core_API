use crate::common::get_pool;
use core_api::database::ressources::can_add_access::CanAddAccessQueryView;
use core_api::endpoints::v1::ressources::AccessType;
use mairie360_api_lib::test_setup::queries_setup::GROUP_OWNER_ID;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let pool = get_pool().await;
    let view = CanAddAccessQueryView::new(
        *GROUP_OWNER_ID.get().unwrap() as u64,
        2,
        1,
        "groups",
        AccessType::Read,
    );
    assert!(view.check(&pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn failure_bad_owner_id() {
    let pool = get_pool().await;
    let view = CanAddAccessQueryView::new(
        (*GROUP_OWNER_ID.get().unwrap() as u64) + 1,
        2,
        1,
        "groups",
        AccessType::Read,
    );
    assert!(!view.check(&pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn failure_bad_ressource_id() {
    let pool = get_pool().await;
    let view = CanAddAccessQueryView::new(
        *GROUP_OWNER_ID.get().unwrap() as u64,
        2,
        2,
        "groups",
        AccessType::Read,
    );
    assert!(!view.check(&pool).await.unwrap());
}

#[tokio::test]
#[serial]
async fn failure_access_type_error() {
    let pool = get_pool().await;
    let view = CanAddAccessQueryView::new(
        *GROUP_OWNER_ID.get().unwrap() as u64,
        1,
        1,
        "groups",
        AccessType::Error,
    );
    assert!(!view.check(&pool).await.unwrap());
}
