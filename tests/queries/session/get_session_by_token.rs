use crate::common::get_pool;
use core_api::database::sessions::{
    create_session::CreateSessionQueryView, get_session_by_token::GetSessionByTokenQueryView,
    Session,
};
use mairie360_api_lib::{
    database::error::DbError, database::query_views::IsSessionTokenValidQueryView,
    error::ApiLibError, test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_get_session_by_token() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_get_session_by_token",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    let _: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_get_session_by_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    let view = GetSessionByTokenQueryView::new("test_get_session_by_token".to_string());
    println!("{}", view);
    assert_eq!(view.get_token(), "test_get_session_by_token");
    let session: Session = pool.fetch_one(&view).await.unwrap();

    println!("{:?}", session);
    assert_eq!(session.user_id(), 1);
    assert_eq!(session.device_info(), "any_device");
    let _ = session.id();
    let _ = session.ip_address();
    let _ = session.created_at();
    let _ = session.expires_at();
    let _ = session.revoked_at();
}

#[tokio::test]
#[serial]
async fn test_get_session_by_unknow_token() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_get_session_by_unknow_token",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    let _: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_get_session_by_unknow_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    let result: Result<Session, _> = pool
        .fetch_one(&GetSessionByTokenQueryView::new("unknow_token".to_string()))
        .await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}
