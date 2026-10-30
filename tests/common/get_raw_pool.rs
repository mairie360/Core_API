use sqlx::{postgres::PgPoolOptions, PgPool};

/// Pool sqlx brut, utilisé uniquement pour la mise en place des fixtures de test (seed, sync de
/// séquence...), avec l'URL superuser de `get_shared_db()` — les requêtes de l'API elles-mêmes
/// passent toutes par `SmartDatabase` (`get_pool`) ou `AppState`, connectés avec le rôle
/// `core_api` (MAIR-556).
pub async fn get_raw_pool(url: String) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&url)
        .await
        .expect("Failed to create Postgres pool")
}
