use crate::common::get_pool;
use core_api::database::auth::login::{LoginUserQueryResultView, LoginUserQueryView};
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use mairie360_api_lib::test_setup::queries_setup::seed_password_hash;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn unset_first_connection_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let email = format!(
        "unset_first_connection_{}@example.com",
        uuid::Uuid::new_v4()
    );
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "Unset",
            "FirstConnection",
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

    let view = UnsetFirstConnectionQueryView::new(user_id as u64, seed_password_hash());
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{result:?}");

    // Le mot de passe doit réellement être remplacé et la première connexion levée.
    let stored: LoginUserQueryResultView = pool
        .fetch_one(&LoginUserQueryView::new(email, String::new()))
        .await
        .unwrap();
    assert_eq!(stored.password(), seed_password_hash());
    assert!(!stored.first_connect());
}

#[tokio::test]
#[serial]
async fn unset_first_connection_bad_user_id() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let view = UnsetFirstConnectionQueryView::new(999_999, seed_password_hash());
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{result:?}");
}
