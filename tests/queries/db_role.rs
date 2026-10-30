//! The code under test connects as `core_api`, never as the superuser (MAIR-556): a superuser
//! ignores the grants of Database's `api_grants.sql`, so a query Core may not run in staging
//! would pass here.

use crate::common::{core_db_url, get_raw_pool};
use serial_test::serial;
use sqlx::error::DatabaseError;

#[tokio::test]
#[serial]
async fn the_code_under_test_runs_as_core_api() {
    let pool = get_raw_pool(core_db_url().await).await;
    let (user, is_superuser): (String, bool) = sqlx::query_as(
        "SELECT current_user::TEXT, rolsuper FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(user, "core_api");
    assert!(!is_superuser);
}

#[tokio::test]
#[serial]
async fn core_api_is_held_to_its_grants() {
    // Core never reads the project domain: the grants refuse it, as in staging.
    let pool = get_raw_pool(core_db_url().await).await;
    let error = sqlx::query("SELECT title FROM projects LIMIT 1")
        .fetch_optional(&pool)
        .await
        .expect_err("core_api must not read projects");

    let code = error.as_database_error().and_then(DatabaseError::code);

    assert_eq!(code.as_deref(), Some("42501"));
}
