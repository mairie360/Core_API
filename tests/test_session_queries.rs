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
    use core_api::database::sessions::create_session::{
        create_session_query, CreateSessionQueryView,
    };
    use core_api::database::sessions::get_session_by_token::{
        get_session_by_token_query, GetSessionByTokenQueryView,
    };
    use core_api::database::sessions::get_sessions_by_user::{
        get_sessions_by_user_query, GetSessionsByUserQueryView,
    };
    use core_api::database::sessions::revoke_previous_session::{
        revoke_previous_session_query, RevokePreviousSessionQueryView,
    };
    use core_api::database::sessions::revoke_session::{
        revoke_session_query, RevokeSessionQueryView,
    };
    use core_api::database::sessions::revoke_session_by_id::{
        revoke_session_by_id_query, RevokeSessionByIdQueryView,
    };
    use core_api::database::sessions::revoke_session_by_token::{
        revoke_session_by_token_query, RevokeSessionByTokenQueryView,
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
                "test_create_session",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
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
                "test_get_sessions_by_user",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
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
                "test_get_sessions_by_user",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
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
                "test_get_session_by_token",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
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
                "test_get_session_by_unknow_token",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
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
                "test_revoke_session_with_id",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
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

        assert!(is_valid);

        let session_id = session.id().clone();

        let result: Result<(), DatabaseError> = revoke_session_by_id_query(
            RevokeSessionByIdQueryView::new(1, session_id),
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
                "test_revoke_session_with_token",
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 1]),
            ),
            pool.clone(),
        )
        .await;

        let is_valid = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_revoke_session_with_token".to_string(),
                std::net::IpAddr::from([0, 0, 0, 1]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        assert!(is_valid);

        let result: Result<(), DatabaseError> = revoke_session_by_token_query(
            RevokeSessionByTokenQueryView::new(1, "test_revoke_session_with_token"),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let is_valid = is_session_token_valid_query(
            IsSessionTokenValidQueryView::new(
                1,
                "test_revoke_session_with_token".to_string(),
                std::net::IpAddr::from([0, 0, 0, 1]),
            ),
            pool.clone(),
        )
        .await
        .unwrap();

        assert!(!is_valid);
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
            RevokeSessionQueryView::new(1, Uuid::new_v4(), "a"),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
            .await
            .unwrap();

        assert!(sessions_2.len() >= sessions.len());
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_previous_session() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

        let sessions = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool.clone())
            .await
            .unwrap();

        let result: Result<(), DatabaseError> = revoke_previous_session_query(
            RevokePreviousSessionQueryView::new(
                1,
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                "",
            ),
            pool.clone(),
        )
        .await;

        assert!(result.is_ok());

        let sessions_2 = get_sessions_by_user_query(GetSessionsByUserQueryView::new(1), pool)
            .await
            .unwrap();

        assert!(sessions_2.len() <= sessions.len());
    }
}

#[cfg(test)]
mod sql_injection_tests {
    use super::*;
    use core_api::database::sessions::create_session::{
        create_session_query, CreateSessionQueryView,
    };

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
                malicious_token,
                "any_device",
                std::net::IpAddr::from([0, 0, 0, 0]),
            ),
            pool,
        )
        .await;

        assert_eq!(result, Ok(()));
    }
}
