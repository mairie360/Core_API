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
    use super::*;
    use chrono::Utc;
    use core_api::database::queries::{
        create_session_query, get_session_by_token_query, get_sessions_by_user_query,
        revoke_session_query,
    };
    use core_api::database::query_views::{
        CreateSessionQueryView, GetSessionByTokenQueryView, GetSessionsByUserQueryView,
        RevokeSessionQueryView,
    };
    use mairie360_api_lib::database::errors::DatabaseError;
    use mairie360_api_lib::database::queries::is_session_token_valid_query;
    use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
    use uuid::Uuid;

    #[tokio::test]
    #[serial]
    async fn test_create_session() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let result: Result<(), DatabaseError> = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_create_session".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let is_valid = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_create_session".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool,
        )
        .await
        .unwrap();

        assert!(is_valid);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_sessions_by_user() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let _ = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_get_sessions_by_user".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        let _ = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_get_sessions_by_user".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        let result = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
            .await
            .unwrap();

        assert!(result.len() > 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_sessions_by_unknow_user() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let _ = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_get_sessions_by_user".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        let _ = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_get_sessions_by_user".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        let result = get_sessions_by_user_query(GetSessionsByUserQueryView::new(2), pool)
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_session_by_token() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let _ = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_get_session_by_token".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        let _ = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_get_session_by_token".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        let result = get_session_by_token_query(
            GetSessionByTokenQueryView::new("test_get_session_by_token".to_string()),
            pool,
        )
        .await
        .unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_session_by_unknow_token() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let _ = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_get_session_by_unknow_token".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        let _ = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_get_session_by_unknow_token".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        let result = get_session_by_token_query(
            GetSessionByTokenQueryView::new("unknow_token".to_string()),
            pool,
        )
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_session_with_id() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let _ = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_revoke_session_with_id".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        let session = get_session_by_token_query(
            GetSessionByTokenQueryView::new("test_revoke_session_with_id".to_string()),
            pool.clone(),
        )
        .await
        .unwrap()
        .unwrap();

        let session_id = session.id().clone();

        let result: Result<(), DatabaseError> = revoke_session_query(
            RevokeSessionQueryView::new(1, None, Some(session_id)),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let is_valid = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_revoke_session_with_id".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        assert!(!is_valid);

        let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
            .await
            .unwrap();

        for session in sessions_2 {
            assert!(session.id() != &session_id);
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_session_with_token() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        // Create a session
        let _ = create_session_query(
            CreateSessionQueryView::new(
                1,
                "test_revoke_session_with_token".to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool.clone(),
        )
        .await;

        let sessions = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool.clone())
            .await
            .unwrap();

        let result: Result<(), DatabaseError> = revoke_session_query(
            RevokeSessionQueryView::new(
                1,
                Some("test_revoke_session_with_token".to_string()),
                None,
            ),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let is_valid = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_revoke_session_with_token".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        assert!(!is_valid);

        let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
            .await
            .unwrap();

        assert!(sessions_2.len() < sessions.len());
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_unknowed_session_with_token_and_id() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let sessions = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool.clone())
            .await
            .unwrap();

        let result: Result<(), DatabaseError> = revoke_session_query(
            RevokeSessionQueryView::new(1, Some("a".to_string()), Some(Uuid::new_v4())),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
            .await
            .unwrap();

        assert!(sessions_2.len() >= sessions.len());
    }
}

#[cfg(test)]
mod sql_injection_tests {
    use super::*;
    use chrono::Utc;
    use core_api::database::queries::create_session_query;
    use core_api::database::query_views::CreateSessionQueryView;

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
    async fn test_injection_create_session() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let malicious_token = "' OR 1=1 --";

        // Create a session
        let result = create_session_query(
            CreateSessionQueryView::new(
                1,
                malicious_token.to_string(),
                "any_device".to_string(),
                std::net::IpAddr::from([0, 0, 0, 0]),
                Utc::now(),
            ),
            pool,
        )
        .await;

        assert_eq!(result, Ok(()));
    }
}
