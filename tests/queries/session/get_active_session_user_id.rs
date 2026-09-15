use crate::common::get_pool;
use core_api::database::sessions::{
    create_session::CreateSessionQueryView,
    get_active_session_user_id::GetActiveSessionUserIdQueryView,
};
use mairie360_api_lib::{
    database::error::DbError, error::ApiLibError, test_setup::queries_setup::get_shared_db,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_get_active_session_user_id_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let token = format!("active_session_user_id_{}", uuid::Uuid::new_v4());

    pool.execute(CreateSessionQueryView::new(
        1,
        &token,
        "any_device",
        std::net::IpAddr::from([0, 0, 0, 0]),
    ))
    .await
    .unwrap();

    let view = GetActiveSessionUserIdQueryView::new(&token);
    println!("{view}");
    assert_eq!(view.get_token_hash(), token);
    let user_id: i32 = pool.fetch_scalar(&view).await.unwrap();

    assert_eq!(user_id, 1);
}

#[tokio::test]
#[serial]
async fn test_get_active_session_user_id_unknown_token() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: Result<i32, _> = pool
        .fetch_scalar(&GetActiveSessionUserIdQueryView::new("unknown_token"))
        .await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}
