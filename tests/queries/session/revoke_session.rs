use crate::common::get_pool;
use core_api::database::sessions::{
    get_sessions_by_user::{get_sessions_by_user_query, GetSessionsByUserQueryView},
    revoke_session::{revoke_session_query, RevokeSessionQueryView},
};
use mairie360_api_lib::{
    database::errors::DatabaseError, test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_revoke_unknowed_session_with_token_and_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let sessions = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool.clone())
        .await
        .unwrap();

    let result: Result<(), DatabaseError> = revoke_session_query(
        RevokeSessionQueryView::new(1, Uuid::new_v4(), "a"),
        pool.clone(),
    )
    .await;

    assert!(result.is_ok());

    let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
        .await
        .unwrap();

    assert!(sessions_2.len() >= sessions.len());
}
