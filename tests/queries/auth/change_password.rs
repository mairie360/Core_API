use crate::common::get_pool;
use core_api::database::auth::change_password::ChangePasswordQueryView;
use core_api::database::auth::login::{LoginUserQueryResultView, LoginUserQueryView};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn change_password_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let email = format!("change_password_{}@example.com", uuid::Uuid::new_v4());
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "Change",
            "Password",
            &email,
            "old_password",
            None,
        ))
        .await
        .unwrap();
    let user_id: i32 = pool
        .fetch_scalar(&GetUserIdQueryView::new(&email))
        .await
        .unwrap();

    let view = ChangePasswordQueryView::new("new_password", user_id as u64);
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{:?}", result);

    let login_result: LoginUserQueryResultView = pool
        .fetch_one(&LoginUserQueryView::new(email, "new_password".to_string()))
        .await
        .unwrap();

    assert_eq!(login_result.password().trim(), "new_password");
}

#[tokio::test]
#[serial]
async fn change_password_bad_user_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = ChangePasswordQueryView::new("new_password", 999_999);
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{:?}", result);
}
