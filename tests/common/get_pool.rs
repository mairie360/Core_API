use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::SmartDatabase;

// Aucune des vues migrées ne déclare de `cache_key`, donc `SmartDatabase` ne touche jamais
// Redis dans ces tests : une URL non joignable suffit (les échecs Redis sont silencieux côté lib).
const TEST_REDIS_URL: &str = "redis://127.0.0.1:6379";

pub async fn get_pool(url: String) -> SmartDatabase {
    let db = Database::new(&url).await;
    let redis = Redis::new(TEST_REDIS_URL);

    SmartDatabase::new(db, redis)
}
