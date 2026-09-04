use crate::common::get_pool;
use core_api::database::sessions::{
    get_sessions_by_user::GetSessionsByUserQueryView, revoke_session::RevokeSessionQueryView,
    Session,
};
use mairie360_api_lib::{error::ApiLibError, test_setup::queries_setup::get_shared_db};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_revoke_unknowed_session_with_token_and_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let sessions: Vec<Session> = pool
        .fetch_all(&GetSessionsByUserQueryView::new(1))
        .await
        .unwrap();

    let view = RevokeSessionQueryView::new(1, Uuid::new_v4(), "a");
    println!("{}", view);
    assert_eq!(view.get_user_id(), 1);
    assert_eq!(view.get_token_hash(), "a");
    let _ = view.get_id();
    let _ = view.get_revoked_at();
    let result: Result<(), ApiLibError> = pool.execute(view).await;

    assert!(result.is_ok());

    let sessions_2: Vec<Session> = pool
        .fetch_all(&GetSessionsByUserQueryView::new(1))
        .await
        .unwrap();

    assert!(sessions_2.len() >= sessions.len());
}
