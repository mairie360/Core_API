use core_api::database::sessions::{
    create_session::CreateSessionQueryView, get_sessions_by_user::GetSessionsByUserQueryView,
    Session,
};
use mairie360_api_lib::{
    database::query_views::IsSessionTokenValidQueryView, test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

use crate::common::get_pool;

#[tokio::test]
#[serial]
async fn test_get_sessions_by_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_get_sessions_by_user",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    let _: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_get_sessions_by_user".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    let view = GetSessionsByUserQueryView::new(1);
    println!("{}", view);
    assert_eq!(view.get_user_id(), 1);
    let result: Vec<Session> = pool.fetch_all(&view).await.unwrap();

    assert!(!result.is_empty());
}

#[tokio::test]
#[serial]
async fn test_get_sessions_by_unknow_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_get_sessions_by_user",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    let _: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_get_sessions_by_user".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    let result: Vec<Session> = pool
        .fetch_all(&GetSessionsByUserQueryView::new(2))
        .await
        .unwrap();

    assert!(result.is_empty());
}
