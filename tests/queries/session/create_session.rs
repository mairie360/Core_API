use crate::common::get_pool;
use core_api::database::sessions::create_session::{create_session_query, CreateSessionQueryView};
use mairie360_api_lib::{
    database::{
        errors::DatabaseError, queries::is_session_token_valid_query,
        query_views::IsSessionTokenValidQueryView,
    },
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let result: Result<(), DatabaseError> = create_session_query(
        CreateSessionQueryView::new(
            1,
            "test_create_session",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await;

    assert!(result.is_ok());

    let is_valid = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_create_session".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool,
    )
    .await
    .unwrap();

    assert!(is_valid);
}

#[tokio::test]
#[serial]
async fn test_injection_create_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let malicious_token = "' OR 1=1 --";

    // Create a session
    let result = create_session_query(
        CreateSessionQueryView::new(
            1,
            malicious_token,
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool,
    )
    .await;

    assert_eq!(result, Ok(()));
}
