use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db_as;

// Aucune des vues migrées ne déclare de `cache_key`, donc `SmartDatabase` ne touche jamais
// Redis dans ces tests : une URL non joignable suffit (les échecs Redis sont silencieux côté lib).
const TEST_REDIS_URL: &str = "redis://127.0.0.1:6379";

/// Postgres role Core connects with in every deployment (Database `api_roles.sql`, MAIR-114).
const CORE_DB_ROLE: &str = "core_api";

/// URL of the shared test database as [`CORE_DB_ROLE`] (MAIR-556): the application code under
/// test (`AppState`, query views) goes through it, so a query the role is not granted fails here
/// with `permission denied` instead of in staging. The fixtures keep the superuser URL of
/// `get_shared_db()` (see `get_raw_pool`).
pub async fn core_db_url() -> String {
    get_shared_db_as(CORE_DB_ROLE).await
}

/// `SmartDatabase` on the shared test database, connected as [`CORE_DB_ROLE`].
pub async fn get_pool() -> SmartDatabase {
    let db = Database::new(&core_db_url().await).await;
    let redis = Redis::new(TEST_REDIS_URL);

    SmartDatabase::new(db, redis)
}
