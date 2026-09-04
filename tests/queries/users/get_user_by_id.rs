use crate::common::get_pool;
use core_api::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_by_id_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserByIdQueryView::new(*ALICE_ID.get().unwrap() as u64);
    println!("{}", view);
    assert_eq!(view.get_id(), *ALICE_ID.get().unwrap() as u64);
    let result: Result<GetUserByIdQueryResultView, _> = pool.fetch_one(&view).await;

    assert!(result.is_ok(), "{:?}", result);
    let result = result.unwrap();
    println!("{:?}", result);
    assert_eq!(result.first_name(), "Alice");
    assert_eq!(result.last_name(), "Smith");
    assert_eq!(result.email(), "alice@example.com");
    assert_eq!(result.status(), "active");
    assert!(!result.is_archived());
    assert_eq!(
        result.json(),
        serde_json::json!({
            "first_name": "Alice",
            "last_name": "Smith",
            "email": "alice@example.com",
            "phone_number": "0102030405",
            "status": "active",
            "is_archived": false,
        })
    );
}

#[tokio::test]
#[serial]
async fn get_user_by_id_bad_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = GetUserByIdQueryView::new(999_999);
    let result: Result<GetUserByIdQueryResultView, _> = pool.fetch_one(&view).await;

    assert!(result.is_err(), "{:?}", result);
}
