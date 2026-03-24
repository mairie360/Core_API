use core_api::database::queries::{about_user_query, login_query, register_query};
use core_api::database::query_views::{
    AboutUserQueryView, LoginUserQueryView, RegisterUserQueryView,
};
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::queries::{does_user_exist_by_id_query, QueryError};
use mairie360_api_lib::database::query_views::DoesUserExistByIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::setup_tests;
use serde_json::json;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[cfg(test)]
mod queries_tests {
    use core_api::database::queries_result_views::LoginUserQueryResultView;

    use super::*;

    async fn get_pool(url: String) -> PgPool {
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&url) // On passe l'URL construite ici
            .await
            .expect("Failed to create Postgres pool")
    }

    // --- TESTS DE LOGIN ---

    #[tokio::test]
    #[serial]
    async fn test_login_user_success() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;
        let result = login_query(
            LoginUserQueryView::new("alice@example.com".to_string(), "password123".to_string()),
            pool,
        )
        .await
        .unwrap();

        assert_eq!(
            result.unwrap(),
            LoginUserQueryResultView::new(1, "password123".to_string())
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_login_user_wrong_password() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

        let result = login_query(
            LoginUserQueryView::new("alice@example.com".to_string(), "wrong_pass".to_string()),
            pool,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_login_user_unknown_email() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

        let result = login_query(
            LoginUserQueryView::new(
                "stranger@danger.com".to_string(),
                "any_password".to_string(),
            ),
            pool,
        )
        .await;

        assert_eq!(result, Ok(None));
    }

    // --- TESTS DE CRÉATION ET CONSULTATION ---

    #[tokio::test]
    #[serial]
    async fn test_register_user_success() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

        let register_result = register_query(
            RegisterUserQueryView::new(
                "John",
                "Doe",
                "new_user@test.com",
                "secure_password",
                Some("0601020304"),
            ),
            pool,
        )
        .await
        .unwrap();

        assert_eq!(register_result, true);
    }

    #[tokio::test]
    #[serial]
    async fn test_about_user_success() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

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
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

        let result = about_user_query(AboutUserQueryView::new(999), pool).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::InvalidId("User ID not found".to_string()))
        );
    }
}

#[cfg(test)]
mod sql_injection_tests {
    use super::*;

    async fn get_pool(url: String) -> PgPool {
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&url) // On passe l'URL construite ici
            .await
            .expect("Failed to create Postgres pool")
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_login_email() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

        let malicious_email = "' OR 1=1 --";

        let result = login_query(
            LoginUserQueryView::new(malicious_email.to_string(), "any_password".to_string()),
            pool,
        )
        .await;

        assert_eq!(result, Ok(None));
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_register_fields() {
        let (_container, host) = setup_tests().await;
        let pool = get_pool(host).await;

        let malicious_name = "John'); DROP TABLE users; --";

        let result = register_query(
            RegisterUserQueryView::new(malicious_name, "Doe", "attacker@test.com", "pass", None),
            pool.clone(),
        )
        .await
        .unwrap();

        assert_eq!(result, true);

        let check_result = does_user_exist_by_id_query(DoesUserExistByIdQueryView::new(1), pool)
            .await
            .unwrap();

        assert_eq!(check_result, true);
    }
}
