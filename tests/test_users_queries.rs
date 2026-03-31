use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

async fn get_pool(url: String) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&url) // On passe l'URL construite ici
        .await
        .expect("Failed to create Postgres pool")
}

#[cfg(test)]
mod queries_tests {
    use core_api::database::queries::about_user_query;
    use core_api::database::query_views::AboutUserQueryView;
    use mairie360_api_lib::database::errors::DatabaseError;
    use mairie360_api_lib::database::queries::QueryError;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_about_user_success() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let result = about_user_query(AboutUserQueryView::new(1), pool)
            .await
            .unwrap();

        let expected_json = json!({
            "first_name": "Alice",
            "last_name": "Smith",
            "email": "alice@example.com",
            "phone_number": "0102030405",
            "status": "active"
        });

        assert_eq!(result.json(), expected_json);
    }

    #[tokio::test]
    #[serial]
    async fn test_about_user_fail() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let result = about_user_query(AboutUserQueryView::new(999), pool).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::InvalidId("User ID not found".to_string()))
        );
    }
}
