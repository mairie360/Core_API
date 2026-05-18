use crate::common::roles::setup_tests;
use crate::common::{get_pool, roles::DELETE_ID};
use core_api::database::roles::delete_role::{delete_role_query, DeleteRoleQueryView};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_delete_role() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteRoleQueryView::new(*DELETE_ID.get().unwrap());
    let result = delete_role_query(view, pool.clone()).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_delete_role_bad_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = DeleteRoleQueryView::new(999);
    let result = delete_role_query(view, pool.clone()).await;

    assert!(result.is_ok());
}
