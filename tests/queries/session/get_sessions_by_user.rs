use core_api::database::sessions::{
    create_session::{create_session_query, CreateSessionQueryView},
    get_sessions_by_user::{get_sessions_by_user_query, GetSessionsByUserQueryView},
};
use mairie360_api_lib::{
    database::{queries::is_session_token_valid_query, query_views::IsSessionTokenValidQueryView},
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

use crate::common::get_pool;

#[tokio::test]
#[serial]
async fn test_get_sessions_by_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = create_session_query(
        CreateSessionQueryView::new(
            1,
            "test_get_sessions_by_user",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await;

    let _ = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_get_sessions_by_user".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await
    .unwrap();

    let result = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
        .await
        .unwrap();

    assert!(result.len() > 0);
}

#[tokio::test]
#[serial]
async fn test_get_sessions_by_unknow_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = create_session_query(
        CreateSessionQueryView::new(
            1,
            "test_get_sessions_by_user",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await;

    let _ = is_session_token_valid_query(
        IsSessionTokenValidQueryView::new(
            1,
            "test_get_sessions_by_user".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ),
        pool.clone(),
    )
    .await
    .unwrap();

    let result = get_sessions_by_user_query(GetSessionsByUserQueryView::new(2), pool)
        .await
        .unwrap();

    assert!(result.is_empty());
}
