use core_api::database::sessions::{
    create_session::CreateSessionQueryView, get_session_by_token::GetSessionByTokenQueryView,
    revoke_session_by_id::RevokeSessionByIdQueryView, Session,
};
use mairie360_api_lib::{
    database::query_views::IsSessionTokenValidQueryView, error::ApiLibError,
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

use crate::common::get_pool;

#[tokio::test]
#[serial]
async fn test_revoke_session_with_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_revoke_session_with_id",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    let session: Session = pool
        .fetch_one(&GetSessionByTokenQueryView::new(
            "test_revoke_session_with_id".to_string(),
        ))
        .await
        .unwrap();

    let is_valid: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_revoke_session_with_id".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    assert!(is_valid);

    let session_id = *session.id();

    let view = RevokeSessionByIdQueryView::new(1, session_id);
    println!("{}", view);
    assert_eq!(view.get_user_id(), 1);
    assert_eq!(*view.get_id(), session_id);
    let _ = view.get_revoked_at();
    let result: Result<(), ApiLibError> = pool.execute(view).await;

    assert!(result.is_ok());

    let is_valid: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_revoke_session_with_id".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    assert!(!is_valid);
}
