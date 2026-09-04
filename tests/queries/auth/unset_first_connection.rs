use crate::common::get_pool;
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn unset_first_connection_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let email = format!(
        "unset_first_connection_{}@example.com",
        uuid::Uuid::new_v4()
    );
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "Unset",
            "FirstConnection",
            &email,
            "password",
            None,
        ))
        .await
        .unwrap();
    let user_id: i32 = pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();

    let view = UnsetFirstConnectionQueryView::new(user_id as u64, "new_password");
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn unset_first_connection_bad_user_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = UnsetFirstConnectionQueryView::new(999_999, "new_password");
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{:?}", result);
}
