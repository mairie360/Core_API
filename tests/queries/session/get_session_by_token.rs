use crate::common::get_pool;
use core_api::database::sessions::{
    create_session::{create_session_query, CreateSessionQueryView},
    get_session_by_token::{get_session_by_token_query, GetSessionByTokenQueryView},
};
use mairie360_api_lib::{
    database::{queries::is_session_token_valid_query, query_views::IsSessionTokenValidQueryView},
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_get_session_by_token() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = create_session_query(
        CreateSessionQueryView::new(
            1,
            "test_get_session_by_token",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await;

    let _ = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_get_session_by_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await
    .unwrap();

    let result = get_session_by_token_query(
        GetSessionByTokenQueryView::new("test_get_session_by_token".to_string()),
        pool,
    )
    .await
    .unwrap();

    assert!(result.is_some());
}

#[tokio::test]
#[serial]
async fn test_get_session_by_unknow_token() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = create_session_query(
        CreateSessionQueryView::new(
            1,
            "test_get_session_by_unknow_token",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await;

    let _ = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_get_session_by_unknow_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await
    .unwrap();

    let result = get_session_by_token_query(
        GetSessionByTokenQueryView::new("unknow_token".to_string()),
        pool,
    )
    .await
    .unwrap();

    assert!(result.is_none());
}
