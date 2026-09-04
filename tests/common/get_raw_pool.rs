use sqlx::{postgres::PgPoolOptions, PgPool};

/// Pool sqlx brut, utilisé uniquement pour la mise en place des fixtures de test (seed, sync de
/// séquence...) — les requêtes de l'API elles-mêmes passent toutes par `SmartDatabase`
/// (`get_pool` dans ce module, malgré son nom historique).
pub async fn get_raw_pool(url: String) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&url)
        .await
        .expect("Failed to create Postgres pool")
}
