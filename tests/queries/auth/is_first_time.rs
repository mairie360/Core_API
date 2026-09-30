use crate::common::get_pool;
use core_api::database::auth::is_first_time::IsFirstTimeQueryView;
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, seed_password_hash};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn is_first_time_reads_the_first_connection_flag() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let email = format!("is_first_time_{}@example.com", uuid::Uuid::new_v4());
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "First",
            "Time",
            &email,
            seed_password_hash(),
            None,
        ))
        .await
        .unwrap();
    let user_id: i32 = pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();

    let view = IsFirstTimeQueryView::new(user_id as u64);
    let before: bool = pool.fetch_scalar(&view).await.unwrap();
    assert!(
        before,
        "a freshly registered account must be in first connection"
    );

    pool.execute(UnsetFirstConnectionQueryView::new(
        user_id as u64,
        seed_password_hash(),
    ))
    .await
    .unwrap();

    let after: bool = pool.fetch_scalar(&view).await.unwrap();
    assert!(
        !after,
        "the flag must be lifted once the password is changed"
    );
}

#[tokio::test]
#[serial]
async fn is_first_time_unknown_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = IsFirstTimeQueryView::new(999_999);
    let result: Result<bool, _> = pool.fetch_scalar(&view).await;

    assert!(result.is_ok(), "{result:?}");
    assert!(!result.unwrap());
}
