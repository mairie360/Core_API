use crate::common::get_pool;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use sqlx::Row;
use tokio::sync::OnceCell;

pub static COUNT: OnceCell<u64> = OnceCell::const_new();
pub static DELETE_ID: OnceCell<u64> = OnceCell::const_new();
pub static PATCH_ID: OnceCell<u64> = OnceCell::const_new();
pub static PATCH_MUTEX: OnceCell<tokio::sync::Mutex<()>> = OnceCell::const_new();

pub async fn setup_tests() {
    if COUNT.get().is_none() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;
        COUNT.set(0).unwrap();
        let _ = sqlx::query(
            "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, $2, true)",
        )
        .bind("Delete")
        .bind("Delete role")
        .execute(&pool)
        .await;
        let delete_id = sqlx::query("SELECT id FROM roles WHERE name = 'Delete'")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i32, _>(0);
        DELETE_ID.set(delete_id as u64).unwrap();
        let _ = sqlx::query(
            "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, $2, true)",
        )
        .bind("Patch")
        .bind("Patch role")
        .execute(&pool)
        .await;
        let patch_id = sqlx::query("SELECT id FROM roles WHERE name = 'Patch'")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i32, _>(0);
        PATCH_ID.set(patch_id as u64).unwrap();
        _ = PATCH_MUTEX.set(tokio::sync::Mutex::new(()));
    }
}
