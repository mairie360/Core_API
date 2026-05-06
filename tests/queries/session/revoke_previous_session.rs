use crate::common::get_pool;
use core_api::database::sessions::{
    get_sessions_by_user::{get_sessions_by_user_query, GetSessionsByUserQueryView},
    revoke_previous_session::{revoke_previous_session_query, RevokePreviousSessionQueryView},
};
use mairie360_api_lib::{
    database::errors::DatabaseError, test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_revoke_previous_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let sessions = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool.clone())
        .await
        .unwrap();

    let result: Result<(), DatabaseError> = revoke_previous_session_query(
        RevokePreviousSessionQueryView::new(
            1,
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            "",
        ),
        pool.clone(),
    )
    .await;

    assert!(result.is_ok());

    let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
        .await
        .unwrap();

    assert!(sessions_2.len() <= sessions.len());
}
