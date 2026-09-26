use crate::common::users::{create_user, unique_marker};
use crate::common::{get_pool, get_raw_pool};
use core_api::database::admin::reset_password::{
    AdminResetPasswordQueryView, AdminResetPasswordResult,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn admin_reset_password_updates_password_and_revokes_sessions() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let user_id = create_user(&pool, "Reset", &unique_marker("Password")).await;
    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, ip_address, device_info) \
         VALUES ($1, $2, '127.0.0.1', 'reset-password-test')",
    )
    .bind(user_id)
    .bind(format!("reset_password_{}", uuid::Uuid::new_v4()))
    .execute(&raw)
    .await
    .unwrap();

    let result: AdminResetPasswordResult = pool
        .fetch_one(&AdminResetPasswordQueryView::new(
            user_id as u64,
            "a-new-password",
        ))
        .await
        .unwrap();

    assert!(result.updated());
    assert_eq!(result.revoked_sessions().len(), 1, "{result:?}");
    let (password, first_connect): (String, bool) =
        sqlx::query_as("SELECT password, first_connect FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&raw)
            .await
            .unwrap();
    assert_eq!(password, "a-new-password");
    assert!(!first_connect);
    let active_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(&raw)
    .await
    .unwrap();
    assert_eq!(active_sessions, 0);
}

#[tokio::test]
#[serial]
async fn admin_reset_password_unknown_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: AdminResetPasswordResult = pool
        .fetch_one(&AdminResetPasswordQueryView::new(999_999, "a-new-password"))
        .await
        .unwrap();

    assert!(!result.updated());
    assert!(result.revoked_sessions().is_empty());
}
