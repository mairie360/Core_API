use crate::common::get_pool;
use core_api::database::sessions::{
    create_session::CreateSessionQueryView, get_sessions::GetSessionsQueryView, Session,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_get_sessions() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let _ = pool
        .execute(CreateSessionQueryView::new(
            1,
            "test_get_sessions",
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    let view = GetSessionsQueryView::new(vec![1, 2]);
    println!("{}", view);
    assert_eq!(view.id(), &[1, 2]);
    let result: Result<Vec<Session>, _> = pool.fetch_all(&view).await;
    assert!(result.is_ok(), "Failed to get sessions: {:?}", result);
    let result = result.unwrap();
    assert!(
        !result.is_empty(),
        "Expected 1 session, got {}",
        result.len()
    );
}
