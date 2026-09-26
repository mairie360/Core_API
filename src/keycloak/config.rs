use mairie360_api_lib::env_manager::get_env_var;

/// Connection settings of the Keycloak realm used for single sign-on.
///
/// Keycloak is optional during the transition from password logins: when the realm URL or the
/// client id is missing, [`KeycloakConfig::from_env`] returns `None` and only the historical
/// `POST /api/v1/auth/login` remains available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeycloakConfig {
    realm_url: String,
    issuer: String,
    client_id: String,
    client_secret: Option<String>,
}

impl KeycloakConfig {
    /// Builds a configuration from its raw parts.
    ///
    /// `realm_url` is the URL Core uses to reach the realm (e.g.
    /// `http://keycloak:8080/realms/mairie360`). `issuer` is the `iss` claim Keycloak writes into
    /// its tokens, which follows Keycloak's public hostname and can therefore differ from
    /// `realm_url` inside a cluster; it defaults to `realm_url`. Trailing slashes are ignored.
    #[must_use]
    pub fn new(
        realm_url: &str,
        issuer: Option<&str>,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> Self {
        let realm_url = realm_url.trim_end_matches('/').to_string();
        let issuer = issuer
            .map(|issuer| issuer.trim_end_matches('/').to_string())
            .filter(|issuer| !issuer.is_empty())
            .unwrap_or_else(|| realm_url.clone());
        Self {
            realm_url,
            issuer,
            client_id: client_id.to_string(),
            client_secret: client_secret
                .filter(|secret| !secret.is_empty())
                .map(ToString::to_string),
        }
    }

    /// Reads `KEYCLOAK_REALM_URL`, `KEYCLOAK_CLIENT_ID`, `KEYCLOAK_CLIENT_SECRET` (optional, for a
    /// confidential client) and `KEYCLOAK_ISSUER` (optional, defaults to the realm URL).
    ///
    /// Returns `None`, which disables Keycloak sign-in, when the realm URL or the client id is
    /// missing or empty.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let realm_url = get_env_var("KEYCLOAK_REALM_URL").filter(|value| !value.is_empty());
        let client_id = get_env_var("KEYCLOAK_CLIENT_ID").filter(|value| !value.is_empty());
        match (realm_url, client_id) {
            (Some(realm_url), Some(client_id)) => Some(Self::new(
                &realm_url,
                get_env_var("KEYCLOAK_ISSUER").as_deref(),
                &client_id,
                get_env_var("KEYCLOAK_CLIENT_SECRET").as_deref(),
            )),
            (None, None) => None,
            _ => {
                eprintln!(
                    "Keycloak sign-in disabled: KEYCLOAK_REALM_URL and KEYCLOAK_CLIENT_ID must both be set."
                );
                None
            }
        }
    }

    #[must_use]
    pub fn realm_url(&self) -> &str {
        &self.realm_url
    }

    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    #[must_use]
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    /// OIDC token endpoint of the realm.
    #[must_use]
    pub fn token_endpoint(&self) -> String {
        format!("{}/protocol/openid-connect/token", self.realm_url)
    }

    /// JSON Web Key Set holding the realm's public signing keys.
    #[must_use]
    pub fn jwks_uri(&self) -> String {
        format!("{}/protocol/openid-connect/certs", self.realm_url)
    }

    /// Base URL of the realm's Admin REST API, derived from the realm URL: its last `/realms/`
    /// segment becomes `/admin/realms/` (`http://keycloak:8080/realms/mairie360` gives
    /// `http://keycloak:8080/admin/realms/mairie360`).
    ///
    /// `None` when the realm URL has no `/realms/` segment: the account migration cannot run,
    /// the sign-in itself is unaffected.
    #[must_use]
    pub fn admin_url(&self) -> Option<String> {
        const REALMS: &str = "/realms/";
        let index = self.realm_url.rfind(REALMS)?;
        let realm = &self.realm_url[index + REALMS.len()..];
        if realm.is_empty() {
            return None;
        }
        Some(format!("{}/admin/realms/{realm}", &self.realm_url[..index]))
    }
}
