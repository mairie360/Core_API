use crate::common::get_pool;
use crate::common::users::{create_user, unique_marker};
use core_api::database::groups::create_group::CreateGroupQueryView;
use core_api::database::users::delete_user::{DeleteUserQueryView, IsUserActiveQueryView};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

async fn is_active(pool: &SmartDatabase, user_id: u64) -> bool {
    pool.fetch_scalar(&IsUserActiveQueryView::new(user_id))
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
async fn test_delete_user_archives_the_account() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let user_id = create_user(&pool, "Delete", &unique_marker("archive")).await as u64;
    assert!(is_active(&pool, user_id).await);

    pool.execute(DeleteUserQueryView::new(user_id))
        .await
        .expect("archiving an active user without resources should succeed");

    assert!(!is_active(&pool, user_id).await);
}

#[tokio::test]
#[serial]
async fn test_is_user_active_is_false_for_an_unknown_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    assert!(!is_active(&pool, 999_999).await);
}

#[tokio::test]
#[serial]
async fn test_delete_user_owning_a_group_is_a_restrict_violation() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let marker = unique_marker("owner");
    let user_id = create_user(&pool, "Owner", &marker).await as u64;
    let _: i32 = pool
        .fetch_scalar(&CreateGroupQueryView::new(user_id, &marker, "Owned group"))
        .await
        .unwrap();

    let error = pool
        .execute(DeleteUserQueryView::new(user_id))
        .await
        .expect_err("a group owner must not be archived");

    assert!(
        matches!(
            &error,
            ApiLibError::Database(DbError::Sqlx(sqlx::Error::Database(db_error)))
                if db_error.code().as_deref() == Some("23001")
        ),
        "expected a restrict_violation, got {error:?}"
    );
    assert!(is_active(&pool, user_id).await);
}
