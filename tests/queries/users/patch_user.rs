use crate::common::get_pool;
use core_api::database::auth::register::RegisterUserQueryView;
use core_api::database::get_user_id::GetUserIdQueryView;
use core_api::database::users::get_user_by_id::{GetUserByIdQueryResultView, GetUserByIdQueryView};
use core_api::database::users::patch_user::PatchUserQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

async fn register_fresh_user(pool: &SmartDatabase, tag: &str) -> u64 {
    let email = format!("patch_user_{}_{}@example.com", tag, uuid::Uuid::new_v4());
    let _: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "Patch",
            "User",
            &email,
            "password",
            Some("0102030405"),
        ))
        .await
        .unwrap();

    pool.fetch_scalar::<i32, _>(&GetUserIdQueryView::new(&email))
        .await
        .unwrap() as u64
}

async fn patch_user(pool: &SmartDatabase, view: PatchUserQueryView) -> Result<(), ApiLibError> {
    if view.is_noop() {
        return Ok(());
    }
    pool.execute(view).await
}

async fn fetch_user(pool: &SmartDatabase, user_id: u64) -> GetUserByIdQueryResultView {
    pool.fetch_one(&GetUserByIdQueryView::new(user_id))
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
async fn patch_user_first_name_only() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "first_name").await;

    let view = PatchUserQueryView::new(user_id, Some("Patched"), None, None, None, None);
    assert!(!view.is_noop());
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);

    let result = fetch_user(&pool, user_id).await;
    assert_eq!(result.first_name(), "Patched");
    assert_eq!(result.last_name(), "User");
}

#[tokio::test]
#[serial]
async fn patch_user_last_name_only() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "last_name").await;

    let view = PatchUserQueryView::new(user_id, None, Some("Patched"), None, None, None);
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);

    let result = fetch_user(&pool, user_id).await;
    assert_eq!(result.last_name(), "Patched");
    assert_eq!(result.first_name(), "Patch");
}

#[tokio::test]
#[serial]
async fn patch_user_email_only() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "email").await;

    let new_email = format!(
        "patch_user_email_patched_{}@example.com",
        uuid::Uuid::new_v4()
    );
    let view = PatchUserQueryView::new(user_id, None, None, Some(&new_email), None, None);
    println!("{}", view);
    assert_eq!(view.email(), Some(new_email.as_str()));
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);

    let result = fetch_user(&pool, user_id).await;
    assert_eq!(result.email(), new_email);
}

#[tokio::test]
#[serial]
async fn patch_user_phone_number_only() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "phone").await;

    let view = PatchUserQueryView::new(user_id, None, None, None, Some("0611223344"), None);
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);

    let result = fetch_user(&pool, user_id).await;
    assert_eq!(result.phone_number(), Some("0611223344"));
}

#[tokio::test]
#[serial]
async fn patch_user_password_only() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "password").await;

    let view = PatchUserQueryView::new(user_id, None, None, None, None, Some("new_password"));
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);
}

#[tokio::test]
#[serial]
async fn patch_user_multiple_fields() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "multiple").await;

    let view = PatchUserQueryView::new(
        user_id,
        Some("Multi"),
        Some("Patched"),
        None,
        Some("0699887766"),
        Some("new_password"),
    );
    println!("{}", view);
    println!("{:?}", view);
    assert_eq!(view.id(), user_id);
    assert_eq!(view.first_name(), Some("Multi"));
    assert_eq!(view.last_name(), Some("Patched"));
    assert_eq!(view.email(), None);
    assert_eq!(view.phone_number(), Some("0699887766"));
    assert_eq!(view.password(), Some("new_password"));
    assert!(!view.is_noop());
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);

    let result = fetch_user(&pool, user_id).await;
    assert_eq!(result.first_name(), "Multi");
    assert_eq!(result.last_name(), "Patched");
    assert_eq!(result.phone_number(), Some("0699887766"));
}

#[tokio::test]
#[serial]
async fn patch_user_noop_when_nothing_provided() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let user_id = register_fresh_user(&pool, "noop").await;

    let view = PatchUserQueryView::new(user_id, None, None, None, None, None);
    assert!(view.is_noop());
    let result = patch_user(&pool, view).await;
    assert!(result.is_ok(), "{:?}", result);

    let result = fetch_user(&pool, user_id).await;
    assert_eq!(result.first_name(), "Patch");
    assert_eq!(result.last_name(), "User");
}

#[tokio::test]
#[serial]
async fn patch_user_bad_user_id_is_noop_free_but_harmless() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let view = PatchUserQueryView::new(999_999, Some("Nobody"), None, None, None, None);
    let result = patch_user(&pool, view).await;

    assert!(result.is_ok(), "{:?}", result);
}
