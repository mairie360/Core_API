use crate::common::get_pool;
use core_api::database::sessions::create_session::CreateSessionQueryView;
use mairie360_api_lib::{
    database::query_views::IsSessionTokenValidQueryView, error::ApiLibError,
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    // Create a session
    let view = CreateSessionQueryView::new(
        1,
        "test_create_session",
        "any_device",
        std::net::IpAddr::from([0, 0, 0, 0]),
    );
    println!("{}", view);
    assert_eq!(view.get_user_id(), 1);
    assert_eq!(view.get_token_hash(), "test_create_session");
    assert_eq!(view.get_device_info(), "any_device");
    assert_eq!(*view.get_ip_address(), std::net::IpAddr::from([0, 0, 0, 0]));
    let result: Result<(), ApiLibError> = pool.execute(view).await;

    assert!(result.is_ok());

    let is_valid: bool = pool
        .fetch_scalar(&IsSessionTokenValidQueryView::new(
            1,
            "test_create_session".to_string(),
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await
        .unwrap();

    assert!(is_valid);
}

#[tokio::test]
#[serial]
async fn test_injection_create_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let malicious_token = "' OR 1=1 --";

    // Create a session
    let result = pool
        .execute(CreateSessionQueryView::new(
            1,
            malicious_token,
            "any_device",
            std::net::IpAddr::from([0, 0, 0, 0]),
        ))
        .await;

    assert!(result.is_ok());
}
