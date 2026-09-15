use crate::common::get_pool;
use core_api::database::sessions::create_session::CreateSessionQueryView;
use core_api::endpoints::v1::auth::create_new_session;
use mairie360_api_lib::{
    database::query_views::IsSessionTokenValidQueryView, error::ApiLibError,
    test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_create_session() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    // Create a session
    let view = CreateSessionQueryView::new(
        1,
        "test_create_session",
        "any_device",
        std::net::IpAddr::from([0, 0, 0, 0]),
    );
    println!("{view}");
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
    let pool = get_pool(host.clone()).await;

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

/// Régression MAIR-124 : deux appareils vus par Core avec la même IP (celle du BFF) et le même
/// `device_info` (User-Agent identique) doivent garder chacun leur session active.
#[tokio::test]
#[serial]
async fn test_create_new_session_keeps_other_device_session_active() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let ip = std::net::IpAddr::from([10, 0, 0, 1]);
    let device_info = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/140.0";
    let first_token = format!("multi_device_first_{}", uuid::Uuid::new_v4());
    let second_token = format!("multi_device_second_{}", uuid::Uuid::new_v4());

    create_new_session(
        &pool,
        CreateSessionQueryView::new(1, &first_token, device_info, ip),
    )
    .await;
    create_new_session(
        &pool,
        CreateSessionQueryView::new(1, &second_token, device_info, ip),
    )
    .await;

    for token in [first_token, second_token] {
        let is_valid: bool = pool
            .fetch_scalar(&IsSessionTokenValidQueryView::new(1, token.clone(), ip))
            .await
            .unwrap();
        assert!(is_valid, "la session {token} a été révoquée");
    }
}
