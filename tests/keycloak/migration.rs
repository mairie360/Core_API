use crate::common::keycloak_mock::{KeycloakMock, CLIENT_ID, CLIENT_SECRET};
use crate::common::users::{create_user, link_identity, unique_marker, user_identities};
use crate::common::{get_pool, get_raw_pool};
use core_api::keycloak::migration::{
    migrate_users, MigrationError, MigrationOptions, MigrationReport, UserMigrationResult,
    UserMigrationStatus, KEYCLOAK_PROVIDER,
};
use core_api::keycloak::{KeycloakAdminClient, KeycloakConfig};
use mairie360_api_lib::test_setup::queries_setup::{get_shared_db, ADMIN_ID, ALICE_ID, BOB_ID};
use serial_test::serial;

async fn run(mock: &KeycloakMock, options: MigrationOptions) -> MigrationReport {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    migrate_users(&pool, &mock.admin_client(), options)
        .await
        .expect("migration run")
}

fn result_of(report: &MigrationReport, user_id: i32) -> &UserMigrationResult {
    report
        .users
        .iter()
        .find(|result| result.user_id == user_id)
        .unwrap_or_else(|| panic!("user {user_id} missing from the report"))
}

#[tokio::test]
#[serial]
async fn test_migration_provisions_every_account_and_links_them() {
    let (_container, host) = get_shared_db().await;
    let raw = get_raw_pool(host.clone()).await;
    let mock = KeycloakMock::start();

    let report = run(&mock, MigrationOptions::default()).await;

    let expected_total: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(report.total as i64, expected_total);
    assert_eq!(
        report.created + report.updated + report.failed,
        report.total
    );
    assert_eq!(report.users.len(), report.total);
    // Accounts sharing an e-mail up to case (created by other tests) cannot both exist in
    // Keycloak: those are the only failures a fresh realm can report.
    for failed in report
        .users
        .iter()
        .filter(|result| result.status == UserMigrationStatus::Failed)
    {
        assert!(
            failed
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("already linked"),
            "{failed:?}"
        );
    }

    // Alice: active administrator, created and linked, role carried over.
    let alice_id = *ALICE_ID.get().unwrap();
    let alice = result_of(&report, alice_id);
    assert_eq!(alice.status, UserMigrationStatus::Created, "{alice:?}");
    assert!(alice.enabled);
    assert!(
        alice.roles_added.contains(&"Admin".to_string()),
        "{alice:?}"
    );
    assert!(!alice.password_email_sent);
    assert_eq!(alice.error, None);
    let keycloak_alice = mock.user(alice.keycloak_id.as_deref().unwrap()).unwrap();
    assert_eq!(keycloak_alice.email, "alice@example.com");
    assert_eq!(keycloak_alice.first_name, "Alice");
    assert!(keycloak_alice.enabled);
    assert!(keycloak_alice.email_verified);
    assert!(keycloak_alice.realm_roles.contains(&"Admin".to_string()));
    assert_eq!(
        user_identities(&raw, alice_id).await,
        vec![(KEYCLOAK_PROVIDER.to_string(), keycloak_alice.id.clone())]
    );

    // Bob: archived, provisioned disabled and still linked.
    let bob_id = *BOB_ID.get().unwrap();
    let bob = result_of(&report, bob_id);
    assert_ne!(bob.status, UserMigrationStatus::Failed, "{bob:?}");
    assert!(!bob.enabled);
    assert!(
        !mock
            .user(bob.keycloak_id.as_deref().unwrap())
            .unwrap()
            .enabled
    );
    assert!(report.disabled >= 1);
    assert_eq!(user_identities(&raw, bob_id).await.len(), 1);

    // Every role of the export exists in the realm.
    let admin = result_of(&report, *ADMIN_ID.get().unwrap());
    assert!(mock
        .user(admin.keycloak_id.as_deref().unwrap())
        .unwrap()
        .realm_roles
        .contains(&"Admin".to_string()));
    assert!(mock.roles().iter().any(|role| role.name == "Admin"));
}

#[tokio::test]
#[serial]
async fn test_migration_replay_creates_no_duplicate() {
    let (_container, host) = get_shared_db().await;
    let raw = get_raw_pool(host.clone()).await;
    let mock = KeycloakMock::start();
    let alice_id = *ALICE_ID.get().unwrap();

    let first = run(&mock, MigrationOptions::default()).await;
    let users_after_first = mock.users();
    let roles_after_first = mock.roles();
    let alice_link = user_identities(&raw, alice_id).await;

    let second = run(&mock, MigrationOptions::default()).await;

    assert_eq!(second.total, first.total);
    assert_eq!(second.created, 0, "{second:?}");
    assert_eq!(second.updated, first.created + first.updated);
    assert_eq!(second.failed, first.failed);
    assert!(second
        .users
        .iter()
        .all(|result| result.roles_added.is_empty()));
    assert_eq!(mock.users(), users_after_first);
    assert_eq!(mock.roles(), roles_after_first);
    assert_eq!(user_identities(&raw, alice_id).await, alice_link);
    assert_eq!(
        result_of(&second, alice_id).keycloak_id,
        result_of(&first, alice_id).keycloak_id
    );
}

#[tokio::test]
#[serial]
async fn test_migration_recreates_the_accounts_of_a_rebuilt_realm() {
    let (_container, host) = get_shared_db().await;
    let raw = get_raw_pool(host.clone()).await;
    let mock = KeycloakMock::start();
    let alice_id = *ALICE_ID.get().unwrap();
    let first = run(&mock, MigrationOptions::default()).await;
    let old_id = result_of(&first, alice_id).keycloak_id.clone().unwrap();

    mock.delete_user(&old_id);
    let second = run(&mock, MigrationOptions::default()).await;

    let alice = result_of(&second, alice_id);
    assert_eq!(alice.status, UserMigrationStatus::Created);
    let new_id = alice.keycloak_id.clone().unwrap();
    assert_ne!(new_id, old_id);
    assert!(alice.roles_added.contains(&"Admin".to_string()));
    assert_eq!(
        user_identities(&raw, alice_id).await,
        vec![(KEYCLOAK_PROVIDER.to_string(), new_id)]
    );
    assert_eq!(second.created, 1);
}

#[tokio::test]
#[serial]
async fn test_migration_adopts_an_account_created_by_hand() {
    let (_container, host) = get_shared_db().await;
    let raw = get_raw_pool(host.clone()).await;
    let mock = KeycloakMock::start();
    let alice_id = *ALICE_ID.get().unwrap();
    let by_hand = mock.seed_user("ALICE@example.com", false);
    let existing_role = mock.seed_role("Admin");

    let report = run(&mock, MigrationOptions::default()).await;

    // A realm role that already exists is reused, never duplicated.
    let admin_roles: Vec<_> = mock
        .roles()
        .into_iter()
        .filter(|role| role.name == "Admin")
        .collect();
    assert_eq!(admin_roles.len(), 1);
    assert_eq!(admin_roles[0].id, existing_role);

    let alice = result_of(&report, alice_id);
    assert_eq!(alice.status, UserMigrationStatus::Updated);
    assert_eq!(alice.keycloak_id.as_deref(), Some(by_hand.as_str()));
    let user = mock.user(&by_hand).unwrap();
    assert!(user.enabled, "synchronised with Core: active");
    assert_eq!(
        (user.first_name.as_str(), user.last_name.as_str()),
        ("Alice", "Smith")
    );
    assert!(user.email_verified);
    assert!(user.realm_roles.contains(&"Admin".to_string()));
    assert_eq!(mock.user_by_email("alice@example.com").unwrap().id, by_hand);
    assert_eq!(
        user_identities(&raw, alice_id).await,
        vec![(KEYCLOAK_PROVIDER.to_string(), by_hand)]
    );
}

#[tokio::test]
#[serial]
async fn test_migration_never_moves_a_linked_identity() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let raw = get_raw_pool(host.clone()).await;
    let marker = unique_marker("migconflict");
    // `first` is processed before `owner` (lower id). Keycloak knows `first`'s e-mail under an
    // account already linked to `owner`: `first` must fail, the link must stay with `owner`.
    let first = create_user(&pool, "First", &marker).await;
    let owner = create_user(&pool, "Owner", &marker).await;
    let first_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(first)
        .fetch_one(&raw)
        .await
        .unwrap();
    let mock = KeycloakMock::start();
    let keycloak_id = mock.seed_user(&first_email, true);
    link_identity(&pool, owner, &keycloak_id).await;

    let report = run(&mock, MigrationOptions::default()).await;

    let failed = result_of(&report, first);
    assert_eq!(failed.status, UserMigrationStatus::Failed);
    assert_eq!(failed.keycloak_id.as_deref(), Some(keycloak_id.as_str()));
    assert_eq!(
        failed.error.as_deref(),
        Some(
            format!(
                "Keycloak account {keycloak_id} is already linked to another Mairie 360 account."
            )
            .as_str()
        )
    );
    assert!(user_identities(&raw, first).await.is_empty());
    let kept = result_of(&report, owner);
    assert_eq!(kept.status, UserMigrationStatus::Updated);
    assert_eq!(kept.keycloak_id.as_deref(), Some(keycloak_id.as_str()));
    assert_eq!(
        user_identities(&raw, owner).await,
        vec![(KEYCLOAK_PROVIDER.to_string(), keycloak_id)]
    );
    assert!(report.failed >= 1);
}

#[tokio::test]
#[serial]
async fn test_migration_sends_password_emails_to_new_enabled_accounts_only() {
    let mock = KeycloakMock::start();
    let options = MigrationOptions {
        send_password_setup_email: true,
    };

    let first = run(&mock, options).await;

    let alice = result_of(&first, *ALICE_ID.get().unwrap());
    assert!(alice.password_email_sent, "{alice:?}");
    assert_eq!(alice.error, None);
    let bob = result_of(&first, *BOB_ID.get().unwrap());
    assert!(!bob.password_email_sent, "archived: no e-mail");
    let emails = mock.password_emails();
    assert!(emails.contains(alice.keycloak_id.as_ref().unwrap()));
    assert!(!emails.contains(bob.keycloak_id.as_ref().unwrap()));
    assert_eq!(
        emails.len(),
        first
            .users
            .iter()
            .filter(|result| result.password_email_sent)
            .count()
    );

    // A replay creates nothing, so it e-mails nobody.
    let second = run(&mock, options).await;
    assert!(second
        .users
        .iter()
        .all(|result| !result.password_email_sent));
    assert_eq!(mock.password_emails(), emails);
}

#[tokio::test]
#[serial]
async fn test_migration_reports_a_failed_password_email_without_failing_the_account() {
    let mock = KeycloakMock::start();
    mock.fail_password_emails();

    let report = run(
        &mock,
        MigrationOptions {
            send_password_setup_email: true,
        },
    )
    .await;

    let alice = result_of(&report, *ALICE_ID.get().unwrap());
    assert_eq!(alice.status, UserMigrationStatus::Created);
    assert!(!alice.password_email_sent);
    assert_eq!(
        alice.error.as_deref(),
        Some("The password set-up e-mail could not be sent: Keycloak is unavailable.")
    );
    assert!(mock.password_emails().is_empty());
}

#[tokio::test]
#[serial]
async fn test_migration_requires_a_confidential_client() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let mock = KeycloakMock::start();
    let admin = KeycloakAdminClient::new(mock.public_config());

    let result = migrate_users(&pool, &admin, MigrationOptions::default()).await;

    assert_eq!(result, Err(MigrationError::NotConfigured));
    assert!(mock.token_requests().is_empty());
    assert!(mock.users().is_empty());
}

#[tokio::test]
#[serial]
async fn test_migration_stops_when_keycloak_refuses_the_service_account() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;

    let denied = KeycloakMock::start();
    denied.deny_service_account();
    assert_eq!(
        migrate_users(&pool, &denied.admin_client(), MigrationOptions::default()).await,
        Err(MigrationError::KeycloakForbidden)
    );

    let forbidden = KeycloakMock::start();
    forbidden.forbid_admin_api();
    assert_eq!(
        migrate_users(
            &pool,
            &forbidden.admin_client(),
            MigrationOptions::default()
        )
        .await,
        Err(MigrationError::KeycloakForbidden)
    );
    assert!(forbidden.users().is_empty());
}

#[tokio::test]
#[serial]
async fn test_migration_stops_when_keycloak_is_unreachable() {
    let (_container, host) = get_shared_db().await;
    let pool = get_pool(host.clone()).await;
    let admin = KeycloakAdminClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        None,
        CLIENT_ID,
        Some(CLIENT_SECRET),
    ));

    let result = migrate_users(&pool, &admin, MigrationOptions::default()).await;

    assert_eq!(result, Err(MigrationError::KeycloakUnavailable));
}

#[test]
fn test_migration_error_messages() {
    assert_eq!(
        MigrationError::NotConfigured.to_string(),
        "Keycloak administration is not configured: a confidential client (KEYCLOAK_CLIENT_SECRET) is required."
    );
    assert_eq!(
        MigrationError::KeycloakForbidden.to_string(),
        "Keycloak refused Core's service account."
    );
    assert_eq!(
        MigrationError::KeycloakUnavailable.to_string(),
        "Keycloak is unavailable."
    );
    assert_eq!(
        MigrationError::Database.to_string(),
        "An error occurred while accessing the database."
    );
}
