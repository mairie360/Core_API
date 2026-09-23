use crate::common::users::{create_user, link_identity, unique_marker};
use crate::common::{get_pool, get_raw_pool};
use core_api::database::admin::sso_export::{ListSsoExportQueryView, SsoExportUser};
use core_api::keycloak::migration::KEYCLOAK_PROVIDER;
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ALICE_ID, BOB_ID};
use serial_test::serial;
use std::collections::BTreeMap;

async fn export(pool: &mairie360_api_lib::smart_db::SmartDatabase) -> Vec<SsoExportUser> {
    pool.fetch_all(&ListSsoExportQueryView::new())
        .await
        .unwrap()
}

fn find(users: &[SsoExportUser], id: i32) -> &SsoExportUser {
    users
        .iter()
        .find(|user| user.id == id)
        .unwrap_or_else(|| panic!("user {id} missing from the export"))
}

#[tokio::test]
#[serial]
async fn test_sso_export_lists_every_account_in_id_order() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;

    let users = export(&pool).await;

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(users.len() as i64, total, "archived accounts included");
    assert!(users.windows(2).all(|pair| pair[0].id < pair[1].id));

    let alice = find(&users, *ALICE_ID.get().unwrap());
    assert_eq!(alice.email, "alice@example.com");
    assert_eq!(
        (alice.first_name.as_str(), alice.last_name.as_str()),
        ("Alice", "Smith")
    );
    assert!(alice.enabled);
    assert!(alice.has_local_password);
    assert!(alice.roles.contains(&"Admin".to_string()));

    let bob = find(&users, *BOB_ID.get().unwrap());
    assert!(!bob.enabled, "archived users are exported disabled");
}

#[tokio::test]
#[serial]
async fn test_sso_export_carries_roles_and_identities() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("export");
    let user_id = create_user(&pool, "Exported", &marker).await;
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) \
         VALUES ($1, (SELECT id FROM roles WHERE name = 'Maire')) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .execute(&raw)
    .await
    .unwrap();
    let subject = format!("sub-{marker}");
    link_identity(&pool, user_id, &subject).await;

    let users = export(&pool).await;

    let user = find(&users, user_id);
    assert_eq!(user.first_name, "Exported");
    assert_eq!(user.last_name, marker);
    assert!(
        user.roles.contains(&"Maire".to_string()),
        "{:?}",
        user.roles
    );
    assert!(
        user.roles.windows(2).all(|pair| pair[0] <= pair[1]),
        "sorted"
    );
    assert_eq!(
        user.identities,
        BTreeMap::from([(KEYCLOAK_PROVIDER.to_string(), subject)])
    );
}

#[tokio::test]
#[serial]
async fn test_sso_export_flags_sso_only_accounts() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("ssoonly");
    let user_id = create_user(&pool, "SsoOnly", &marker).await;
    assert!(find(&export(&pool).await, user_id).has_local_password);
    assert!(find(&export(&pool).await, user_id).identities.is_empty());

    sqlx::query("UPDATE users SET password = NULL WHERE id = $1")
        .bind(user_id)
        .execute(&raw)
        .await
        .unwrap();

    assert!(!find(&export(&pool).await, user_id).has_local_password);
}

#[test]
fn test_sso_export_view_display() {
    assert_eq!(
        ListSsoExportQueryView::default().to_string(),
        "ListSsoExportQueryView"
    );
}
