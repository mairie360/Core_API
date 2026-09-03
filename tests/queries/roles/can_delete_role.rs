use crate::common::get_pool;
use crate::common::roles::{setup_tests, CAN_DELETE_ID};
use core_api::database::roles::can_delete_role::CanDeleteRoleQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_delete_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = CanDeleteRoleQueryView::new(*CAN_DELETE_ID.get().unwrap());
    println!("{}", view);
    assert_eq!(view.id(), *CAN_DELETE_ID.get().unwrap());
    let result: bool = pool.fetch_scalar(&view).await.unwrap();

    assert!(result);
}

#[tokio::test]
#[serial]
async fn test_can_delete_role_bad_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result: Result<bool, _> = pool.fetch_scalar(&CanDeleteRoleQueryView::new(999)).await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, ApiLibError::Database(DbError::NotFound)));
}
