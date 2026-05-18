use crate::common::get_pool;
use core_api::database::ressources::get_ressource_type_id::{
    get_ressource_type_id_query, GetRessourceTypeIdQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("users");
    let result = get_ressource_type_id_query(view, pool).await.unwrap();
    assert_eq!(result, 1, "{}", format!("Expected 1, got {}", result));
}

#[tokio::test]
#[serial]
async fn failure() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let view = GetRessourceTypeIdQueryView::new("invalid");
    assert!(get_ressource_type_id_query(view, pool).await.is_err());
}
