use core_api::keycloak::KeycloakConfig;
use serial_test::serial;

const ENV_VARS: [&str; 4] = [
    "KEYCLOAK_REALM_URL",
    "KEYCLOAK_CLIENT_ID",
    "KEYCLOAK_CLIENT_SECRET",
    "KEYCLOAK_ISSUER",
];

/// Runs `test` with exactly the given Keycloak variables set, then clears them all.
fn with_env(vars: &[(&str, &str)], test: impl FnOnce()) {
    for name in ENV_VARS {
        std::env::remove_var(name);
    }
    for (name, value) in vars {
        std::env::set_var(name, value);
    }
    test();
    for name in ENV_VARS {
        std::env::remove_var(name);
    }
}

#[test]
fn test_endpoints_are_derived_from_realm_url() {
    let config = KeycloakConfig::new(
        "https://auth.mairie360.test/realms/mairie360/",
        None,
        "core-api",
        Some("secret"),
    );

    assert_eq!(
        config.realm_url(),
        "https://auth.mairie360.test/realms/mairie360"
    );
    assert_eq!(config.issuer(), config.realm_url());
    assert_eq!(
        config.token_endpoint(),
        "https://auth.mairie360.test/realms/mairie360/protocol/openid-connect/token"
    );
    assert_eq!(
        config.jwks_uri(),
        "https://auth.mairie360.test/realms/mairie360/protocol/openid-connect/certs"
    );
    assert_eq!(config.client_id(), "core-api");
    assert_eq!(config.client_secret(), Some("secret"));
}

#[test]
fn test_explicit_issuer_overrides_realm_url() {
    let config = KeycloakConfig::new(
        "http://keycloak:8080/realms/mairie360",
        Some("https://auth.mairie360.test/realms/mairie360/"),
        "core-api",
        None,
    );

    assert_eq!(
        config.issuer(),
        "https://auth.mairie360.test/realms/mairie360"
    );
    assert_eq!(
        config.token_endpoint(),
        "http://keycloak:8080/realms/mairie360/protocol/openid-connect/token"
    );
}

#[test]
fn test_empty_optional_values_are_ignored() {
    let config = KeycloakConfig::new(
        "https://auth.mairie360.test/realms/mairie360",
        Some(""),
        "core-api",
        Some(""),
    );

    assert_eq!(config.issuer(), config.realm_url());
    assert_eq!(config.client_secret(), None);
}

#[test]
#[serial(keycloak_env)]
fn test_from_env_reads_all_variables() {
    with_env(
        &[
            (
                "KEYCLOAK_REALM_URL",
                "http://keycloak:8080/realms/mairie360",
            ),
            ("KEYCLOAK_CLIENT_ID", "core-api"),
            ("KEYCLOAK_CLIENT_SECRET", "secret"),
            (
                "KEYCLOAK_ISSUER",
                "https://auth.mairie360.test/realms/mairie360",
            ),
        ],
        || {
            assert_eq!(
                KeycloakConfig::from_env(),
                Some(KeycloakConfig::new(
                    "http://keycloak:8080/realms/mairie360",
                    Some("https://auth.mairie360.test/realms/mairie360"),
                    "core-api",
                    Some("secret"),
                ))
            );
        },
    );
}

#[test]
#[serial(keycloak_env)]
fn test_from_env_without_variables_disables_keycloak() {
    with_env(&[], || assert_eq!(KeycloakConfig::from_env(), None));
}

#[test]
#[serial(keycloak_env)]
fn test_from_env_with_partial_configuration_disables_keycloak() {
    with_env(
        &[(
            "KEYCLOAK_REALM_URL",
            "http://keycloak:8080/realms/mairie360",
        )],
        || assert_eq!(KeycloakConfig::from_env(), None),
    );
    with_env(&[("KEYCLOAK_CLIENT_ID", "core-api")], || {
        assert_eq!(KeycloakConfig::from_env(), None);
    });
    with_env(
        &[
            ("KEYCLOAK_REALM_URL", ""),
            ("KEYCLOAK_CLIENT_ID", "core-api"),
        ],
        || assert_eq!(KeycloakConfig::from_env(), None),
    );
}

#[test]
fn test_admin_url_is_derived_from_realm_url() {
    let config = KeycloakConfig::new(
        "https://auth.mairie360.test/realms/mairie360/",
        None,
        "core-api",
        Some("secret"),
    );
    assert_eq!(
        config.admin_url().as_deref(),
        Some("https://auth.mairie360.test/admin/realms/mairie360")
    );

    // Legacy `/auth` context path: the segment is inserted before the last `/realms/`.
    let legacy = KeycloakConfig::new(
        "http://keycloak:8080/auth/realms/mairie360",
        None,
        "core-api",
        None,
    );
    assert_eq!(
        legacy.admin_url().as_deref(),
        Some("http://keycloak:8080/auth/admin/realms/mairie360")
    );
}

#[test]
fn test_admin_url_requires_a_realm_segment() {
    let no_segment = KeycloakConfig::new("https://auth.mairie360.test/mairie360", None, "c", None);
    assert_eq!(no_segment.admin_url(), None);

    let no_realm = KeycloakConfig::new("https://auth.mairie360.test/realms/", None, "c", None);
    assert_eq!(no_realm.admin_url(), None);
}
