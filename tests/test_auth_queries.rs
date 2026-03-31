use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

async fn sync_user_sequence(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Cette requête récupère le nom de la séquence associée à la colonne 'id' 
    // de la table 'users' et la met à jour avec le MAX(id) actuel.
    let sync_query = r#"
        SELECT setval(
            pg_get_serial_sequence('users', 'id'), 
            COALESCE(MAX(id), 1), 
            max(id) IS NOT NULL
        ) FROM users;
    "#;

    sqlx::query(sync_query)
        .execute(pool)
        .await?;

    Ok(())
}

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
    

    #[cfg(test)]
    mod login {
        use super::*;

        use core_api::database::queries::login_query;
        use core_api::database::queries_result_views::LoginUserQueryResultView;
        use core_api::database::query_views::LoginUserQueryView;
        use serial_test::serial;

        #[tokio::test]
        #[serial]
        async fn test_login_user_success() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.to_string()).await;
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
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.to_string()).await;

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
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.to_string()).await;

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
    }

    #[cfg(test)]
    mod register {
        use super::*;
        use core_api::database::queries::register_query;
        use core_api::database::query_views::RegisterUserQueryView;
        use serial_test::serial;

        #[tokio::test]
        #[serial]
        async fn test_register_user_success() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.to_string()).await;
            sync_user_sequence(&pool).await.unwrap();

            let unique_email = format!("test_{}@test.com", uuid::Uuid::new_v4());
            
            let register_result = register_query(
                RegisterUserQueryView::new(
                    "John",
                    "Doe",
                    &unique_email,
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
        async fn test_register_user_duplicate_email() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.to_string()).await;
            sync_user_sequence(&pool).await.unwrap();
            
            let unique_email = format!("test_{}@test.com", uuid::Uuid::new_v4());

            let _ = register_query(
                RegisterUserQueryView::new(
                    "John",
                    "Doe",
                    &unique_email,
                    "secure_password",
                    Some("0601020304"),
                ),
                pool.clone(),
            )
            .await;
            
            let register_result = register_query(
                RegisterUserQueryView::new(
                    "John",
                    "Doe",
                    &unique_email,
                    "secure_password",
                    Some("0601020304"),
                ),
                pool,
            )
            .await;

            assert!(register_result.is_err());
        }
    }
}

#[cfg(test)]
mod sql_injection_tests {
    use super::*;
    use core_api::database::queries::{login_query, register_query};
    use core_api::database::query_views::{LoginUserQueryView, RegisterUserQueryView};
    use mairie360_api_lib::database::queries::does_user_exist_by_id_query;
    use mairie360_api_lib::database::query_views::DoesUserExistByIdQueryView;

    async fn get_pool(url: String) -> PgPool {
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&url)
            .await
            .expect("Failed to create Postgres pool")
    }

    #[tokio::test]
    #[serial]
    async fn test_injection_login_email() {
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;

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
        let (_container, host) = get_shared_db().await;
        let pool = get_pool(host.to_string()).await;
        sync_user_sequence(&pool).await.unwrap();

        let malicious_name = "John'); DROP TABLE users; --";
        
        let unique_email = format!("test_{}@test.com", uuid::Uuid::new_v4());

        let result = register_query(
            RegisterUserQueryView::new(malicious_name, "Doe", &unique_email, "pass", None),
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
