//! MAIR-238: `IsRenameForbiddenQueryView` mirrors the `protect_role_names` trigger, which the
//! role PATCH route relies on to answer 403 instead of a database error.
use crate::common::{get_pool, get_raw_pool};
use core_api::database::roles::is_rename_forbidden::IsRenameForbiddenQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
use sqlx::Row;

async fn insert_role(name: &str, can_be_deleted: bool) -> u64 {
    let (_container, host) = get_shared_db().await;
    let pool = get_raw_pool(host.clone()).await;
    sqlx::query("INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, $1, $2)")
        .bind(name)
        .bind(can_be_deleted)
        .execute(&pool)
        .await
        .unwrap();
    let id = sqlx::query("SELECT id FROM roles WHERE name = $1")
        .bind(name)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i32, _>(0);
    u64::try_from(id).unwrap()
}

async fn is_forbidden(id: u64, name: &str) -> Result<bool, ApiLibError> {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool().await;
    let view = IsRenameForbiddenQueryView::new(id, name);
    println!("{view}");
    pool.fetch_scalar(&view).await
}

#[tokio::test]
#[serial]
async fn mair_238_system_role_rename_is_forbidden() {
    let id = insert_role("Rename guard system", false).await;
    assert!(is_forbidden(id, "Another name").await.unwrap());
}

#[tokio::test]
#[serial]
async fn mair_238_system_role_same_name_is_allowed() {
    let id = insert_role("Rename guard system same", false).await;
    assert!(!is_forbidden(id, "Rename guard system same").await.unwrap());
}

#[tokio::test]
#[serial]
async fn mair_238_deletable_role_rename_is_allowed() {
    let id = insert_role("Rename guard custom", true).await;
    assert!(!is_forbidden(id, "Another name").await.unwrap());
}

#[tokio::test]
#[serial]
async fn mair_238_unknown_role_is_not_found() {
    let err = is_forbidden(999_999, "Anything").await.unwrap_err();
    assert!(matches!(err, ApiLibError::Database(DbError::NotFound)));
}
