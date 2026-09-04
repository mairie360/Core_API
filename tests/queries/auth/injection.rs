use crate::common::{get_pool, get_raw_pool};
use core_api::database::auth::login::{LoginUserQueryResultView, LoginUserQueryView};
use core_api::database::auth::register::RegisterUserQueryView;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::database::query_views::DoesUserExistByIdQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
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

    sqlx::query(sync_query).execute(pool).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_injection_login_email() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;

    let malicious_email = "' OR 1=1 --";

    let result: Result<LoginUserQueryResultView, _> = pool
        .fetch_one(&LoginUserQueryView::new(
            malicious_email.to_string(),
            "any_password".to_string(),
        ))
        .await;

    assert!(matches!(
        result,
        Err(ApiLibError::Database(DbError::NotFound))
    ));
}

#[tokio::test]
#[serial]
async fn test_injection_register_fields() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let raw_pool = get_raw_pool(host.to_string()).await;
    sync_user_sequence(&raw_pool).await.unwrap();

    let malicious_name = "John'); DROP TABLE users; --";

    let unique_email = format!("test_{}@test.com", uuid::Uuid::new_v4());

    let result: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            malicious_name,
            "Doe",
            &unique_email,
            "pass",
            None,
        ))
        .await
        .unwrap();

    assert!(result);

    let check_result: bool = pool
        .fetch_scalar(&DoesUserExistByIdQueryView::new(1))
        .await
        .unwrap();

    assert!(check_result);
}
