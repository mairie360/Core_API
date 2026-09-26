use crate::common::users::{archive_user, create_user, link_identity, unique_marker};
use crate::common::{get_pool, get_raw_pool};
use core_api::database::auth::resolve_identity::{
    ResolveUserIdentityQueryResultView, ResolveUserIdentityQueryView,
};
use core_api::keycloak::migration::KEYCLOAK_PROVIDER;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

async fn resolve(
    pool: &mairie360_api_lib::smart_db::SmartDatabase,
    provider: &str,
    subject: &str,
) -> Option<i32> {
    pool.fetch_one::<ResolveUserIdentityQueryResultView, _>(&ResolveUserIdentityQueryView::new(
        provider, subject,
    ))
    .await
    .unwrap()
    .user_id()
}

#[tokio::test]
#[serial]
async fn test_resolve_identity_returns_the_linked_active_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let marker = unique_marker("resolve");
    let user_id = create_user(&pool, "Resolved", &marker).await;
    let subject = format!("sub-{marker}");
    link_identity(&pool, user_id, &subject).await;

    assert_eq!(
        resolve(&pool, KEYCLOAK_PROVIDER, &subject).await,
        Some(user_id)
    );
}

#[tokio::test]
#[serial]
async fn test_resolve_identity_matches_provider_and_subject() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let marker = unique_marker("resolvematch");
    let user_id = create_user(&pool, "Resolved", &marker).await;
    let subject = format!("sub-{marker}");
    link_identity(&pool, user_id, &subject).await;

    assert_eq!(resolve(&pool, "other-idp", &subject).await, None);
    assert_eq!(
        resolve(&pool, KEYCLOAK_PROVIDER, "unknown-subject").await,
        None
    );
}

#[tokio::test]
#[serial]
async fn test_resolve_identity_refuses_an_archived_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("resolvearch");
    let user_id = create_user(&pool, "Archived", &marker).await;
    let subject = format!("sub-{marker}");
    link_identity(&pool, user_id, &subject).await;

    archive_user(&raw, user_id).await;

    assert_eq!(resolve(&pool, KEYCLOAK_PROVIDER, &subject).await, None);
    // The link survives archiving.
    let links: i64 = sqlx::query_scalar("SELECT count(*) FROM user_identities WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(links, 1);
}

#[test]
fn test_resolve_identity_view_display_and_accessors() {
    let view = ResolveUserIdentityQueryView::new("keycloak", "sub-42");

    assert_eq!(view.provider(), "keycloak");
    assert_eq!(view.subject(), "sub-42");
    assert_eq!(
        view.to_string(),
        "ResolveUserIdentityQueryView: provider = keycloak, subject = sub-42"
    );
    assert_eq!(
        ResolveUserIdentityQueryResultView::new(Some(7)).user_id(),
        Some(7)
    );
}
