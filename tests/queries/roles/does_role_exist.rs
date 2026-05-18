use crate::common::{get_pool, roles::setup_tests};
use core_api::database::roles::does_role_exist::{does_role_exist_query, DoesRoleExistQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_does_role_exist_true() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = does_role_exist_query(DoesRoleExistQueryView::new(1), pool)
        .await
        .unwrap();

    assert!(result);
}

#[tokio::test]
#[serial]
async fn test_does_role_exist_false() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = does_role_exist_query(DoesRoleExistQueryView::new(999), pool)
        .await
        .unwrap();

    assert!(!result);
}
