use crate::common::get_pool;
use crate::common::roles::{setup_tests, DELETE_ID};
use core_api::database::roles::can_delete_role::{can_delete_role_query, CanDeleteRoleQueryView};
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::queries::QueryError;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_can_delete_role_success() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result =
        can_delete_role_query(CanDeleteRoleQueryView::new(*DELETE_ID.get().unwrap()), pool)
            .await
            .unwrap();

    assert!(result);
}

#[tokio::test]
#[serial]
async fn test_can_delete_role_bad_id() {
    setup_tests().await;
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let result = can_delete_role_query(CanDeleteRoleQueryView::new(999), pool).await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err, DatabaseError::Query(QueryError::NoResults));
}
