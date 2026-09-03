use crate::common::get_pool;
use core_api::database::ressources::get_ressource_type_id::GetRessourceTypeIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("users");
    println!("{}", view);
    assert_eq!(view.ressource_type(), "users");
    let result: i32 = pool.fetch_scalar(&view).await.unwrap();
    assert_eq!(result, 1, "Expected 1, got {}", result);
}

#[tokio::test]
#[serial]
async fn failure() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("invalid");
    let result: Result<i32, _> = pool.fetch_scalar(&view).await;
    assert!(result.is_err());
}
