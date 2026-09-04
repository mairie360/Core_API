use crate::common::get_pool;
use core_api::database::sessions::get_active_session::GetActiveSessionQueryView;
use core_api::database::sessions::Session;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;
use std::net::IpAddr;

#[tokio::test]
#[serial]
async fn get_active_session_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetActiveSessionQueryView::new(
        *ALICE_ID.get().unwrap() as u64,
        "127.0.0.1".parse::<IpAddr>().unwrap(),
        "Mozilla/5.0 (TestRunner)",
    );
    let result: Result<Session, _> = pool.fetch_one(&view).await;

    assert!(result.is_ok(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn get_active_session_no_match() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetActiveSessionQueryView::new(
        *ALICE_ID.get().unwrap() as u64,
        "127.0.0.1".parse::<IpAddr>().unwrap(),
        "some_unknown_device",
    );
    let result: Result<Session, _> = pool.fetch_one(&view).await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}
