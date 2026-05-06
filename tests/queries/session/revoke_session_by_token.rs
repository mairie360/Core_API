use core_api::database::sessions::{
    create_session::{create_session_query, CreateSessionQueryView},
    revoke_session_by_token::{revoke_session_by_token_query, RevokeSessionByTokenQueryView},
};
use mairie360_api_lib::{
    database::{
        errors::DatabaseError, queries::is_session_token_valid_query,
        query_views::IsSessionTokenValidQueryView,
    },
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

use crate::common::get_pool;

#[tokio::test]
#[serial]
async fn test_revoke_session_with_token() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = create_session_query(
        CreateSessionQueryView::new(
            1,
            "test_revoke_session_with_token",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 1]),
        ),
        pool.clone(),
    )
    .await;

    let is_valid = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_revoke_session_with_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 1]),
        ),
        pool.clone(),
    )
    .await
    .unwrap();

    assert!(is_valid);

    let result: Result<(), DatabaseError> = revoke_session_by_token_query(
        RevokeSessionByTokenQueryView::new(1, "test_revoke_session_with_token"),
        pool.clone(),
    )
    .await;

    assert!(result.is_ok());

    let is_valid = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_revoke_session_with_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 1]),
        ),
        pool.clone(),
    )
    .await
    .unwrap();

    assert!(!is_valid);
}
