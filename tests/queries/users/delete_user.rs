use crate::common::get_pool;
use crate::common::roles::setup_tests;
use core_api::database::users::delete_user::{delete_user_query, DeleteUserQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_delete_user_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteUserQueryView::new(2);

    let result = delete_user_query(view, pool).await;

    assert!(
        result.is_ok(),
        "delete_user_query should succeed with valid user_id, {:?}",
        result
    );
}

#[tokio::test]
#[serial]
async fn test_delete_user_bad_user_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteUserQueryView::new(999);

    let result = delete_user_query(view, pool).await;

    assert!(
        result.is_ok(),
        "delete_user_query should succeed with bad user_id, {:?}",
        result
    );
}
