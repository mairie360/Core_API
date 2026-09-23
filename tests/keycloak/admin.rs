use crate::common::keycloak_mock::{KeycloakMock, CLIENT_ID, CLIENT_SECRET};
use core_api::keycloak::{
    KeycloakAdminClient, KeycloakAdminError, KeycloakConfig, KeycloakRole, KeycloakUserProfile,
};

fn profile(email: &str, enabled: bool) -> KeycloakUserProfile {
    KeycloakUserProfile {
        email: email.to_string(),
        first_name: "Claire".to_string(),
        last_name: "Martin".to_string(),
        enabled,
    }
}

#[tokio::test]
async fn test_admin_client_requests_a_service_account_token_once() {
    let mock = KeycloakMock::start();
    mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();
    assert!(admin.is_configured());

    admin
        .find_user_by_email("claire.martin@mairie360.test")
        .await
        .unwrap();
    admin
        .find_user_by_email("claire.martin@mairie360.test")
        .await
        .unwrap();

    assert_eq!(mock.admin_token_grants(), 1);
    let requests = mock.token_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["grant_type"], "client_credentials");
    assert_eq!(requests[0]["client_id"], CLIENT_ID);
    assert_eq!(requests[0]["client_secret"], CLIENT_SECRET);
}

#[tokio::test]
async fn test_admin_client_refreshes_an_expired_token_once() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();
    admin.get_user(&id).await.unwrap();

    mock.expire_admin_token();
    let user = admin.get_user(&id).await.unwrap();

    assert_eq!(user.map(|user| user.id), Some(id));
    assert_eq!(mock.admin_token_grants(), 2);
}

#[tokio::test]
async fn test_admin_client_reports_a_refused_service_account() {
    let mock = KeycloakMock::start();
    mock.deny_service_account();

    let result = mock.admin_client().find_user_by_email("x@y.z").await;

    assert_eq!(result, Err(KeycloakAdminError::Forbidden));
}

#[tokio::test]
async fn test_admin_client_reports_a_forbidden_admin_api() {
    let mock = KeycloakMock::start();
    mock.forbid_admin_api();

    let result = mock.admin_client().find_user_by_email("x@y.z").await;

    assert_eq!(result, Err(KeycloakAdminError::Forbidden));
}

#[tokio::test]
async fn test_admin_client_requires_a_confidential_client() {
    let mock = KeycloakMock::start();
    let admin = KeycloakAdminClient::new(mock.public_config());

    assert!(!admin.is_configured());
    assert_eq!(
        admin.find_user_by_email("x@y.z").await,
        Err(KeycloakAdminError::NotConfigured)
    );
    assert!(mock.token_requests().is_empty());
}

#[tokio::test]
async fn test_admin_client_requires_a_realm_segment() {
    let admin = KeycloakAdminClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/mairie360",
        None,
        CLIENT_ID,
        Some(CLIENT_SECRET),
    ));

    assert!(!admin.is_configured());
    assert_eq!(
        admin.get_user("any").await,
        Err(KeycloakAdminError::NotConfigured)
    );
}

#[tokio::test]
async fn test_admin_client_unreachable_keycloak_is_unavailable() {
    let admin = KeycloakAdminClient::new(KeycloakConfig::new(
        "http://127.0.0.1:9/realms/test",
        None,
        CLIENT_ID,
        Some(CLIENT_SECRET),
    ));

    assert_eq!(
        admin.find_user_by_email("x@y.z").await,
        Err(KeycloakAdminError::Unavailable)
    );
}

#[tokio::test]
async fn test_create_user_returns_the_id_from_location() {
    let mock = KeycloakMock::start();
    let admin = mock.admin_client();

    let id = admin
        .create_user(&profile("Claire.Martin@mairie360.test", true))
        .await
        .unwrap();

    let user = mock.user(&id).expect("user created in the realm");
    assert_eq!(user.username, "claire.martin@mairie360.test");
    assert_eq!(user.email, "claire.martin@mairie360.test");
    assert_eq!(user.first_name, "Claire");
    assert_eq!(user.last_name, "Martin");
    assert!(user.enabled);
    assert!(user.email_verified, "the e-mail comes from Core: verified");
    assert!(user.realm_roles.is_empty());

    assert_eq!(
        admin
            .create_user(&profile("claire.martin@mairie360.test", true))
            .await,
        Err(KeycloakAdminError::AlreadyExists)
    );
}

#[tokio::test]
async fn test_find_user_by_email_ignores_case_and_returns_none_when_unknown() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("Jean.Dupont@mairie360.test", true);
    let admin = mock.admin_client();

    let found = admin
        .find_user_by_email("jean.dupont@MAIRIE360.test")
        .await
        .unwrap();
    assert_eq!(found.map(|user| user.id), Some(id));

    assert_eq!(
        admin.find_user_by_email("nobody@mairie360.test").await,
        Ok(None)
    );
}

#[tokio::test]
async fn test_get_user_returns_none_for_a_vanished_user() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("jean.dupont@mairie360.test", true);
    let admin = mock.admin_client();
    assert!(admin.get_user(&id).await.unwrap().is_some());

    mock.delete_user(&id);

    assert_eq!(admin.get_user(&id).await, Ok(None));
}

#[tokio::test]
async fn test_update_user_overwrites_the_profile() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();

    admin
        .update_user(&id, &profile("claire.martin@mairie360.test", false))
        .await
        .unwrap();

    let user = mock.user(&id).unwrap();
    assert!(!user.enabled);
    assert_eq!(
        (user.first_name.as_str(), user.last_name.as_str()),
        ("Claire", "Martin")
    );
    assert!(user.email_verified);

    assert_eq!(
        admin
            .update_user("vanished", &profile("claire.martin@mairie360.test", true))
            .await,
        Err(KeycloakAdminError::NotFound)
    );
}

#[tokio::test]
async fn test_realm_roles_are_created_once_and_mapped() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();
    assert_eq!(admin.realm_role("Maire").await, Ok(None));

    admin.create_realm_role("Maire", "Mayor").await.unwrap();
    // A second creation is a no-op, not an error.
    admin.create_realm_role("Maire", "Mayor").await.unwrap();
    let role = admin.realm_role("Maire").await.unwrap().expect("role");
    assert_eq!(role.name, "Maire");
    assert_eq!(mock.roles().len(), 1);

    assert_eq!(admin.user_realm_roles(&id).await, Ok(Vec::new()));
    admin
        .add_user_realm_roles(&id, std::slice::from_ref(&role))
        .await
        .unwrap();
    assert_eq!(admin.user_realm_roles(&id).await, Ok(vec![role.clone()]));
    assert_eq!(mock.user(&id).unwrap().realm_roles, vec!["Maire"]);

    // Role names with spaces travel percent-encoded in the path.
    admin.create_realm_role("Can Delete", "").await.unwrap();
    assert!(admin.realm_role("Can Delete").await.unwrap().is_some());

    assert_eq!(
        admin
            .add_user_realm_roles("vanished", std::slice::from_ref(&role))
            .await,
        Err(KeycloakAdminError::NotFound)
    );
    let unknown = KeycloakRole {
        id: "no-such-id".to_string(),
        name: "Ghost".to_string(),
    };
    assert_eq!(
        admin.add_user_realm_roles(&id, &[unknown]).await,
        Err(KeycloakAdminError::NotFound)
    );
}

#[tokio::test]
async fn test_send_password_setup_email() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();

    admin.send_password_setup_email(&id).await.unwrap();
    assert_eq!(mock.password_emails(), vec![id.clone()]);

    assert_eq!(
        admin.send_password_setup_email("vanished").await,
        Err(KeycloakAdminError::NotFound)
    );

    mock.fail_password_emails();
    assert_eq!(
        admin.send_password_setup_email(&id).await,
        Err(KeycloakAdminError::Unavailable)
    );
}

#[tokio::test]
async fn test_set_enabled_only_touches_the_flag() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();

    admin.set_enabled(&id, false).await.unwrap();
    let user = mock.user(&id).unwrap();
    assert!(!user.enabled);
    assert_eq!(user.first_name, "Seeded", "profile untouched");

    admin.set_enabled(&id, true).await.unwrap();
    assert!(mock.user(&id).unwrap().enabled);

    assert_eq!(
        admin.set_enabled("vanished", false).await,
        Err(KeycloakAdminError::NotFound)
    );
}

#[tokio::test]
async fn test_delete_user_is_idempotent() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();

    admin.delete_user(&id).await.unwrap();
    assert!(mock.users().is_empty());
    assert_eq!(mock.deletions(), vec![id.clone()]);

    assert_eq!(admin.delete_user(&id).await, Ok(()), "already gone");
}

#[tokio::test]
async fn test_logout_user_ends_the_sessions() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    let admin = mock.admin_client();

    admin.logout_user(&id).await.unwrap();
    assert_eq!(mock.logouts(), vec![id]);

    assert_eq!(
        admin.logout_user("vanished").await,
        Err(KeycloakAdminError::NotFound)
    );
}

#[tokio::test]
async fn test_remove_user_realm_roles_unmaps_only_the_given_roles() {
    let mock = KeycloakMock::start();
    let id = mock.seed_user("claire.martin@mairie360.test", true);
    mock.map_role(&id, "Maire");
    mock.map_role(&id, "User");
    let admin = mock.admin_client();
    let maire = admin.realm_role("Maire").await.unwrap().unwrap();

    admin
        .remove_user_realm_roles(&id, std::slice::from_ref(&maire))
        .await
        .unwrap();
    assert_eq!(
        mock.user(&id).unwrap().realm_roles,
        vec!["User".to_string()]
    );

    assert_eq!(
        admin
            .remove_user_realm_roles("vanished", std::slice::from_ref(&maire))
            .await,
        Err(KeycloakAdminError::NotFound)
    );
}

#[test]
fn test_admin_error_messages() {
    assert_eq!(
        KeycloakAdminError::NotConfigured.to_string(),
        "Keycloak administration is not configured: a confidential client (KEYCLOAK_CLIENT_SECRET) is required."
    );
    assert_eq!(
        KeycloakAdminError::Forbidden.to_string(),
        "Keycloak refused Core's service account."
    );
    assert_eq!(
        KeycloakAdminError::NotFound.to_string(),
        "The Keycloak resource does not exist."
    );
    assert_eq!(
        KeycloakAdminError::AlreadyExists.to_string(),
        "The Keycloak resource already exists."
    );
    assert_eq!(
        KeycloakAdminError::Unavailable.to_string(),
        "Keycloak is unavailable."
    );
}
