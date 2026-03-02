use core_api::database::queries::{about_user_query, login_query, register_query};
use core_api::database::query_views::{
    AboutUserQueryView, LoginUserQueryView, RegisterUserQueryView,
};
use mairie360_api_lib::database::db_interface::QueryResultView;
use mairie360_api_lib::database::errors::DatabaseError;
use mairie360_api_lib::database::queries::{does_user_exist_by_id_query, QueryError};
use mairie360_api_lib::database::queries_result_views::utils::QueryResult;
use mairie360_api_lib::database::query_views::DoesUserExistByIdQueryView;
use mairie360_api_lib::test_setup::queries_setup::setup_tests;
use serde_json::json;
use serial_test::serial;

#[cfg(test)]
mod queries_tests {
    use super::*;

    // --- TESTS DE LOGIN ---

    #[tokio::test]
    #[serial]
    async fn test_login_user_success() {
        let _container = setup_tests().await;
        let result = login_query(LoginUserQueryView::new(
            "alice@example.com".to_string(),
            "password123".to_string(),
        ))
        .await
        .unwrap();

        assert_eq!(result.get_result(), QueryResult::U64(1));
    }

    #[tokio::test]
    #[serial]
    async fn test_login_user_wrong_password() {
        let _container = setup_tests().await;

        let result = login_query(LoginUserQueryView::new(
            "alice@example.com".to_string(),
            "wrong_pass".to_string(),
        ))
        .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::InvalidPassword("alice@example.com".to_string()))
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_login_user_unknown_email() {
        let _container = setup_tests().await;

        let result = login_query(LoginUserQueryView::new(
            "stranger@danger.com".to_string(),
            "any_password".to_string(),
        ))
        .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::EmailNotFound("stranger@danger.com".to_string()))
        );
    }

    // --- TESTS DE CRÉATION ET CONSULTATION ---

    #[tokio::test]
    #[serial]
    async fn test_register_user_success() {
        let _container = setup_tests().await;

        let register_result = register_query(RegisterUserQueryView::new(
            "John".to_string(),
            "Doe".to_string(),
            "new_user@test.com".to_string(),
            "secure_password".to_string(),
            Some("0601020304".to_string()),
        ))
        .await
        .unwrap();

        assert_eq!(register_result.get_result(), QueryResult::Result(Ok(())));
    }

    #[tokio::test]
    #[serial]
    async fn test_about_user_success() {
        let _container = setup_tests().await;

        let result = about_user_query(AboutUserQueryView::new(1)).await.unwrap();

        let expected_json = json!({
            "first_name": "Alice",
            "last_name": "Smith",
            "email": "alice@example.com",
            "phone": "0102030405",
            "status": "active"
        });

        assert_eq!(result.get_result(), QueryResult::JSON(expected_json));
    }

    #[tokio::test]
    #[serial]
    async fn test_about_user_fail() {
        let _container = setup_tests().await;

        let result = about_user_query(AboutUserQueryView::new(999)).await;

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

    #[tokio::test]
    #[serial]
    async fn test_injection_login_email() {
        let _container = setup_tests().await;

        let malicious_email = "' OR 1=1 --";

        let result = login_query(LoginUserQueryView::new(
            malicious_email.to_string(),
            "any_password".to_string(),
        ))
        .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(
            err,
            DatabaseError::Query(QueryError::EmailNotFound(malicious_email.to_string()))
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_register_fields() {
        let _container = setup_tests().await;

        let malicious_name = "John'); DROP TABLE users; --";

        let result = register_query(RegisterUserQueryView::new(
            malicious_name.to_string(),
            "Doe".to_string(),
            "attacker@test.com".to_string(),
            "pass".to_string(),
            None,
        ))
        .await
        .unwrap();

        assert_eq!(result.get_result(), QueryResult::Result(Ok(())));

        let check_result = does_user_exist_by_id_query(DoesUserExistByIdQueryView::new(1))
            .await
            .unwrap();

        assert_eq!(check_result.get_result(), QueryResult::Boolean(true));
    }
}
