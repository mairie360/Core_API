use crate::common::{get_pool, get_raw_pool};
use core_api::database::auth::register::RegisterUserQueryView;
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
async fn test_register_user_success() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let raw_pool = get_raw_pool(host.to_string()).await;
    sync_user_sequence(&raw_pool).await.unwrap();

    let unique_email = format!("test_{}@test.com", uuid::Uuid::new_v4());

    let register_result: bool = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "John",
            "Doe",
            &unique_email,
            "secure_password",
            Some("0601020304"),
        ))
        .await
        .unwrap();

    assert!(register_result);
}

#[tokio::test]
#[serial]
async fn test_register_user_duplicate_email() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.to_string()).await;
    let raw_pool = get_raw_pool(host.to_string()).await;
    sync_user_sequence(&raw_pool).await.unwrap();

    let unique_email = format!("test_{}@test.com", uuid::Uuid::new_v4());

    let _: Result<bool, _> = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "John",
            "Doe",
            &unique_email,
            "secure_password",
            Some("0601020304"),
        ))
        .await;

    let register_result: Result<bool, _> = pool
        .fetch_scalar(&RegisterUserQueryView::new(
            "John",
            "Doe",
            &unique_email,
            "secure_password",
            Some("0601020304"),
        ))
        .await;

    assert!(register_result.is_err());
}
