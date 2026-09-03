use crate::common::get_pool;
use core_api::database::sessions::get_active_sessions::GetActiveSessionsQueryView;
use core_api::database::sessions::Session;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_active_sessions_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetActiveSessionsQueryView::new(*ALICE_ID.get().unwrap() as u64);
    println!("{}", view);
    assert_eq!(view.get_user_id(), *ALICE_ID.get().unwrap() as u64);
    let result: Result<Vec<Session>, _> = pool.fetch_all(&view).await;

    assert!(result.is_ok(), "{:?}", result);
    let sessions = result.unwrap();
    assert!(!sessions.is_empty());
    assert_eq!(sessions[0].user_id(), *ALICE_ID.get().unwrap());
}

#[tokio::test]
#[serial]
async fn get_active_sessions_unknown_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetActiveSessionsQueryView::new(999_999);
    let result: Result<Vec<Session>, _> = pool.fetch_all(&view).await;

    assert!(result.is_ok(), "{:?}", result);
    assert!(result.unwrap().is_empty());
}
