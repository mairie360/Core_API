use crate::common::get_pool;
use crate::common::get_raw_pool;
use crate::common::users::{create_user, unique_marker};
use core_api::database::users::delete_user::DeleteUserQueryView;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

async fn is_archived(raw: &sqlx::PgPool, user_id: i32) -> bool {
    sqlx::query_scalar("SELECT is_archived FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(raw)
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
async fn test_delete_user_archives_the_account() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let user_id = create_user(&pool, "Deleted", &unique_marker("del")).await;

    let view = DeleteUserQueryView::new(user_id as u64);
    assert_eq!(view.user_id(), user_id as u64);
    println!("{view}");
    let result = pool.execute(view).await;

    assert!(result.is_ok(), "{result:?}");
    assert!(is_archived(&raw, user_id).await, "soft-deleted, row kept");
}

#[tokio::test]
#[serial]
async fn test_delete_user_unknown_or_archived_matches_no_row() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let user_id = create_user(&pool, "Twice", &unique_marker("del")).await;
    pool.execute(DeleteUserQueryView::new(user_id as u64))
        .await
        .unwrap();

    let again = pool.execute(DeleteUserQueryView::new(user_id as u64)).await;
    let unknown = pool.execute(DeleteUserQueryView::new(999_999)).await;

    assert!(again.is_ok(), "{again:?}");
    assert!(unknown.is_ok(), "{unknown:?}");
    assert!(is_archived(&raw, user_id).await);
}

#[tokio::test]
#[serial]
async fn test_delete_user_refuses_a_group_owner() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let user_id = create_user(&pool, "Owner", &unique_marker("del")).await;
    sqlx::query("INSERT INTO groups (owner_id, name) VALUES ($1, $2)")
        .bind(user_id)
        .bind(format!("Group {}", unique_marker("grp")))
        .execute(&raw)
        .await
        .unwrap();

    let result = pool.execute(DeleteUserQueryView::new(user_id as u64)).await;

    assert!(result.is_err(), "the schema protects group owners");
    assert!(!is_archived(&raw, user_id).await);
}
