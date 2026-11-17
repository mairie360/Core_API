use crate::common::get_raw_pool;
use sqlx::Row;
use tokio::sync::OnceCell;

pub static COUNT: OnceCell<u64> = OnceCell::const_new();
pub static DELETE_ID: OnceCell<u64> = OnceCell::const_new();
pub static CAN_DELETE_ID: OnceCell<u64> = OnceCell::const_new();
pub static PATCH_ID: OnceCell<u64> = OnceCell::const_new();
pub static PATCH_MUTEX: OnceCell<tokio::sync::Mutex<()>> = OnceCell::const_new();

/// Inserts the three fixture roles in the database of the calling test (MAIR-1002: every test has
/// its own, so the rows must exist in each of them). The copies start from the same seeded template,
/// so the ids are the same in all of them: the statics are set by the first call and only read after.
pub async fn setup_tests() {
    let url = crate::common::own_db_url().await;
    let pool = get_raw_pool(url).await;
    let _ = COUNT.set(0);
    for (name, description, id) in [
        ("Delete", "Delete role", &DELETE_ID),
        ("Can Delete", "Can Delete role", &CAN_DELETE_ID),
        ("Patch", "Patch role", &PATCH_ID),
    ] {
        sqlx::query(
            "INSERT INTO roles (name, description, can_be_deleted) VALUES ($1, $2, true) ON CONFLICT DO NOTHING",
        )
        .bind(name)
        .bind(description)
        .execute(&pool)
        .await
        .unwrap();
        let role_id = sqlx::query("SELECT id FROM roles WHERE name = $1")
            .bind(name)
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i32, _>(0);
        let role_id = u64::try_from(role_id).unwrap();
        let _ = id.set(role_id);
        assert_eq!(
            *id.get().unwrap(),
            role_id,
            "the seeded copies must give the same role ids"
        );
    }
    let _ = PATCH_MUTEX.set(tokio::sync::Mutex::new(()));
}
