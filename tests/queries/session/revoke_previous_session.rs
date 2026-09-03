use crate::common::get_pool;
use core_api::database::sessions::{
    get_sessions_by_user::GetSessionsByUserQueryView,
    revoke_previous_session::RevokePreviousSessionQueryView, Session,
};
use mairie360_api_lib::{error::ApiLibError, test_setup::queries_setup::get_shared_db};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_revoke_previous_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let sessions: Vec<Session> = pool
        .fetch_all(&GetSessionsByUserQueryView::new(1))
        .await
        .unwrap();

    let view = RevokePreviousSessionQueryView::new(
        1,
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
        "",
    );
    println!("{}", view);
    assert_eq!(view.get_user_id(), 1);
    let _ = view.get_ip();
    let _ = view.get_device_info();
    let _ = view.get_revoked_at();
    let result: Result<(), ApiLibError> = pool.execute(view).await;

    assert!(result.is_ok());

    let sessions_2: Vec<Session> = pool
        .fetch_all(&GetSessionsByUserQueryView::new(1))
        .await
        .unwrap();

    assert!(sessions_2.len() <= sessions.len());
}
