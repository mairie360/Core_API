use core_api::database::sessions::{
    create_session::CreateSessionQueryView, revoke_session_by_token::RevokeSessionByTokenQueryView,
};
use mairie360_api_lib::{
    database::query_views::IsSessionTokenValidQueryView, error::ApiLibError,
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
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_revoke_session_with_token",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 1]),
        ))
        .await;

    let is_valid: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_revoke_session_with_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 1]),
        ))
        .await
        .unwrap();

    assert!(is_valid);

    let view = RevokeSessionByTokenQueryView::new(1, "test_revoke_session_with_token");
    println!("{}", view);
    assert_eq!(view.get_user_id(), 1);
    assert_eq!(view.get_token_hash(), "test_revoke_session_with_token");
    let _ = view.get_revoked_at();
    let result: Result<(), ApiLibError> = pool.execute(view).await;

    assert!(result.is_ok());

    let is_valid: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_revoke_session_with_token".to_string(),
            std::net::IpAddr::from([0, 0, 0, 1]),
        ))
        .await
        .unwrap();

    assert!(!is_valid);
}
