use crate::common::get_raw_pool;
use sqlx::Row;
use tokio::sync::OnceCell;

pub static COUNT: OnceCell<u64> = OnceCell::const_new();
pub static DELETE_ID: OnceCell<u64> = OnceCell::const_new();
pub static CAN_DELETE_ID: OnceCell<u64> = OnceCell::const_new();
pub static PATCH_ID: OnceCell<u64> = OnceCell::const_new();
pub static PATCH_MUTEX: OnceCell<tokio::sync::Mutex<()>> = OnceCell::const_new();

/// Runs once even when tests call it at the same time (they run in parallel since MAIR-1002): the
/// others wait for the fixtures instead of reading ids that are not set yet.
static SETUP: OnceCell<()> = OnceCell::const_new();

pub async fn setup_tests() {
    SETUP
        .get_or_init(|| async {
            setup_fixtures().await;
        })
        .await;
}

async fn setup_fixtures() {
    {
        let host = crate::common::own_db_url().await;
        let pool = get_raw_pool(host.clone()).await;
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
        .bind("Can Delete")
        .bind("Can Delete role")
        .execute(&pool)
        .await;
        let can_delete_id = sqlx::query("SELECT id FROM roles WHERE name = 'Can Delete'")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i32, _>(0);
        CAN_DELETE_ID.set(can_delete_id as u64).unwrap();
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
