use crate::common::users::{create_user, unique_marker, user_identities};
use crate::common::{get_pool, get_raw_pool};
use core_api::database::auth::link_identity::LinkUserIdentityQueryView;
use core_api::keycloak::migration::KEYCLOAK_PROVIDER;
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_link_identity_is_replayable_and_relinks_a_new_subject() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("link");
    let user_id = create_user(&pool, "Linked", &marker).await;
    let subject = format!("sub-{marker}");

    let first: i32 = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            user_id,
            KEYCLOAK_PROVIDER,
            &subject,
        ))
        .await
        .unwrap();
    let replay: i32 = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            user_id,
            KEYCLOAK_PROVIDER,
            &subject,
        ))
        .await
        .unwrap();
    assert_eq!(replay, first, "a replay returns the existing link");
    assert_eq!(
        user_identities(&raw, user_id).await,
        vec![(KEYCLOAK_PROVIDER.to_string(), subject)]
    );

    // The provider re-issued the account: the row is updated, not duplicated.
    let new_subject = format!("sub-{marker}-v2");
    let relinked: i32 = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            user_id,
            KEYCLOAK_PROVIDER,
            &new_subject,
        ))
        .await
        .unwrap();
    assert_eq!(relinked, first);
    assert_eq!(
        user_identities(&raw, user_id).await,
        vec![(KEYCLOAK_PROVIDER.to_string(), new_subject)]
    );
}

#[tokio::test]
#[serial]
async fn test_link_identity_allows_one_identity_per_provider() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("linkmulti");
    let user_id = create_user(&pool, "Multi", &marker).await;

    for provider in ["franceconnect", KEYCLOAK_PROVIDER] {
        let _: i32 = pool
            .fetch_scalar(&LinkUserIdentityQueryView::new(
                user_id,
                provider,
                &format!("{provider}-{marker}"),
            ))
            .await
            .unwrap();
    }

    assert_eq!(
        user_identities(&raw, user_id).await,
        vec![
            (
                "franceconnect".to_string(),
                format!("franceconnect-{marker}")
            ),
            (KEYCLOAK_PROVIDER.to_string(), format!("keycloak-{marker}")),
        ]
    );
}

#[tokio::test]
#[serial]
async fn test_link_identity_refuses_a_subject_held_by_another_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let marker = unique_marker("linkconflict");
    let owner = create_user(&pool, "Owner", &marker).await;
    let other = create_user(&pool, "Other", &marker).await;
    let subject = format!("sub-{marker}");
    let _: i32 = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            owner,
            KEYCLOAK_PROVIDER,
            &subject,
        ))
        .await
        .unwrap();

    let result: Result<i32, _> = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            other,
            KEYCLOAK_PROVIDER,
            &subject,
        ))
        .await;

    assert!(
        matches!(
            result,
            Err(ApiLibError::Database(DbError::UniqueViolation(_)))
        ),
        "{result:?}"
    );
}

#[tokio::test]
#[serial]
async fn test_link_identity_refuses_an_unknown_user() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let result: Result<i32, _> = pool
        .fetch_scalar(&LinkUserIdentityQueryView::new(
            i32::MAX,
            KEYCLOAK_PROVIDER,
            "ghost",
        ))
        .await;

    assert!(
        matches!(
            result,
            Err(ApiLibError::Database(DbError::ForeignKeyViolation(_)))
        ),
        "{result:?}"
    );
}

#[test]
fn test_link_identity_view_display_and_accessors() {
    let view = LinkUserIdentityQueryView::new(42, "keycloak", "sub-42");

    assert_eq!(view.user_id(), 42);
    assert_eq!(view.provider(), "keycloak");
    assert_eq!(view.subject(), "sub-42");
    assert_eq!(
        view.to_string(),
        "LinkUserIdentityQueryView: user_id = 42, provider = keycloak, subject = sub-42"
    );
}
