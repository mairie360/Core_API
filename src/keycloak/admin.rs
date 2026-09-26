use super::config::KeycloakConfig;
use reqwest::header::LOCATION;
use reqwest::{Method, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::sync::Mutex;

/// Timeout of every HTTP call to the Admin API.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Failure while calling the Keycloak Admin REST API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeycloakAdminError {
    /// Core has no client secret (public client) or the realm URL has no `/realms/` segment:
    /// no service-account token can be obtained.
    NotConfigured,
    /// Keycloak refused Core's service account (`401`/`403`, or `400 unauthorized_client`):
    /// wrong secret, service accounts disabled on the client, or missing `realm-management`
    /// roles (`manage-users`, `view-realm`, `manage-realm`).
    Forbidden,
    /// The resource does not exist (`404`).
    NotFound,
    /// The resource already exists (`409`).
    AlreadyExists,
    /// Keycloak could not be reached, or answered with an unexpected status or body.
    Unavailable,
}

impl Display for KeycloakAdminError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "Keycloak administration is not configured: a confidential client (KEYCLOAK_CLIENT_SECRET) is required."
            ),
            Self::Forbidden => write!(f, "Keycloak refused Core's service account."),
            Self::NotFound => write!(f, "The Keycloak resource does not exist."),
            Self::AlreadyExists => write!(f, "The Keycloak resource already exists."),
            Self::Unavailable => write!(f, "Keycloak is unavailable."),
        }
    }
}

impl std::error::Error for KeycloakAdminError {}

/// Keycloak user as the Admin API returns it (the fields of `UserRepresentation` Core uses).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUser {
    /// Keycloak user id, the `sub` claim of the user's tokens.
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

/// Profile written to Keycloak when creating or updating a user. The e-mail also serves as
/// username, and is always marked verified: it comes from the Mairie 360 account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeycloakUserProfile {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    /// `false` disables the account: Keycloak refuses to sign it in.
    pub enabled: bool,
}

impl KeycloakUserProfile {
    fn representation(&self, username: bool) -> Value {
        let mut body = json!({
            "email": self.email,
            "firstName": self.first_name,
            "lastName": self.last_name,
            "enabled": self.enabled,
            "emailVerified": true
        });
        if username {
            body["username"] = json!(self.email);
        }
        body
    }
}

/// Realm role, as listed and mapped by the Admin API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeycloakRole {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Client of the realm's Admin REST API.
///
/// Authenticated as the service account of Core's confidential client (`client_credentials`
/// grant). The token is fetched on first use and refreshed once when Keycloak answers `401`.
pub struct KeycloakAdminClient {
    config: KeycloakConfig,
    admin_url: Option<Url>,
    http: reqwest::Client,
    token: Mutex<Option<String>>,
}

impl KeycloakAdminClient {
    /// # Panics
    ///
    /// Panics if the HTTP client cannot be built (TLS backend initialisation failure), which
    /// only happens on a broken host.
    #[must_use]
    pub fn new(config: KeycloakConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .expect("Failed to build the Keycloak Admin HTTP client");
        let admin_url = config.admin_url().and_then(|url| Url::parse(&url).ok());
        Self {
            config,
            admin_url,
            http,
            token: Mutex::new(None),
        }
    }

    #[must_use]
    pub const fn config(&self) -> &KeycloakConfig {
        &self.config
    }

    /// `true` when a service-account token can be requested: confidential client and a realm
    /// URL the Admin API URL can be derived from.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.admin_url.is_some() && self.config.client_secret().is_some()
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url, KeycloakAdminError> {
        let mut url = self
            .admin_url
            .clone()
            .ok_or(KeycloakAdminError::NotConfigured)?;
        url.path_segments_mut()
            .map_err(|()| KeycloakAdminError::NotConfigured)?
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }

    /// Returns the cached service-account token, fetching a new one first when `refresh` is
    /// set or none is cached yet.
    async fn token(&self, refresh: bool) -> Result<String, KeycloakAdminError> {
        let mut cached = self.token.lock().await;
        if !refresh {
            if let Some(token) = cached.as_ref() {
                return Ok(token.clone());
            }
        }
        let token = self.fetch_token().await?;
        *cached = Some(token.clone());
        drop(cached);
        Ok(token)
    }

    async fn fetch_token(&self) -> Result<String, KeycloakAdminError> {
        if self.admin_url.is_none() {
            return Err(KeycloakAdminError::NotConfigured);
        }
        let secret = self
            .config
            .client_secret()
            .ok_or(KeycloakAdminError::NotConfigured)?;
        let response = self
            .http
            .post(self.config.token_endpoint())
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.config.client_id()),
                ("client_secret", secret),
            ])
            .send()
            .await
            .map_err(|e| {
                eprintln!("Keycloak token endpoint unreachable: {e}");
                KeycloakAdminError::Unavailable
            })?;

        let status = response.status();
        // `400 unauthorized_client` when service accounts are disabled on the client, `401
        // invalid_client` for a wrong secret.
        if matches!(
            status,
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
        ) {
            eprintln!(
                "Keycloak refused the service account of {}",
                self.config.client_id()
            );
            return Err(KeycloakAdminError::Forbidden);
        }
        if !status.is_success() {
            eprintln!("Keycloak token endpoint answered {status}");
            return Err(KeycloakAdminError::Unavailable);
        }
        let body: TokenResponse = response.json().await.map_err(|e| {
            eprintln!("Unreadable Keycloak token response: {e}");
            KeycloakAdminError::Unavailable
        })?;
        Ok(body.access_token)
    }

    /// Sends an authenticated request. A `401` refreshes the token and retries once; `403`
    /// (or a second `401`) is [`KeycloakAdminError::Forbidden`], `404`
    /// [`KeycloakAdminError::NotFound`], `409` [`KeycloakAdminError::AlreadyExists`].
    async fn send(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
    ) -> Result<reqwest::Response, KeycloakAdminError> {
        let mut refresh = false;
        loop {
            let token = self.token(refresh).await?;
            let mut request = self
                .http
                .request(method.clone(), url.clone())
                .bearer_auth(&token);
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = request.send().await.map_err(|e| {
                eprintln!("Keycloak Admin API unreachable: {e}");
                KeycloakAdminError::Unavailable
            })?;
            match response.status() {
                StatusCode::UNAUTHORIZED if !refresh => refresh = true,
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    eprintln!("Keycloak Admin API refused {method} {}", url.path());
                    return Err(KeycloakAdminError::Forbidden);
                }
                StatusCode::NOT_FOUND => return Err(KeycloakAdminError::NotFound),
                StatusCode::CONFLICT => return Err(KeycloakAdminError::AlreadyExists),
                status if status.is_success() => return Ok(response),
                status => {
                    eprintln!(
                        "Keycloak Admin API answered {status} on {method} {}",
                        url.path()
                    );
                    return Err(KeycloakAdminError::Unavailable);
                }
            }
        }
    }

    async fn read_json<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T, KeycloakAdminError> {
        response.json().await.map_err(|e| {
            eprintln!("Unreadable Keycloak Admin API response: {e}");
            KeycloakAdminError::Unavailable
        })
    }

    /// Finds the user whose e-mail is `email` (case-insensitive, as Keycloak stores e-mails in
    /// lower case).
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<KeycloakUser>, KeycloakAdminError> {
        let mut url = self.endpoint(&["users"])?;
        url.query_pairs_mut()
            .append_pair("email", email)
            .append_pair("exact", "true");
        let users: Vec<KeycloakUser> =
            Self::read_json(self.send(Method::GET, url, None).await?).await?;
        Ok(users.into_iter().find(|user| {
            user.email
                .as_deref()
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(email))
        }))
    }

    /// Returns the user with Keycloak id `id`, `None` if it no longer exists.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn get_user(&self, id: &str) -> Result<Option<KeycloakUser>, KeycloakAdminError> {
        match self
            .send(Method::GET, self.endpoint(&["users", id])?, None)
            .await
        {
            Ok(response) => Self::read_json(response).await.map(Some),
            Err(KeycloakAdminError::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Creates a user without any credential and returns its Keycloak id.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::AlreadyExists`] if the username or e-mail is already taken, plus
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn create_user(
        &self,
        profile: &KeycloakUserProfile,
    ) -> Result<String, KeycloakAdminError> {
        let response = self
            .send(
                Method::POST,
                self.endpoint(&["users"])?,
                Some(&profile.representation(true)),
            )
            .await?;
        // Keycloak answers `201` with the new resource in `Location`, and no body.
        response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|location| location.trim_end_matches('/').rsplit('/').next())
            .filter(|id| !id.is_empty())
            .map(ToString::to_string)
            .ok_or_else(|| {
                eprintln!("Keycloak created a user without a Location header");
                KeycloakAdminError::Unavailable
            })
    }

    /// Overwrites the profile (names, e-mail, enabled flag) of user `id`.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotFound`] if the user vanished, plus
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn update_user(
        &self,
        id: &str,
        profile: &KeycloakUserProfile,
    ) -> Result<(), KeycloakAdminError> {
        self.send(
            Method::PUT,
            self.endpoint(&["users", id])?,
            Some(&profile.representation(false)),
        )
        .await
        .map(|_| ())
    }

    /// Returns the realm role named `name`, `None` if the realm has none.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn realm_role(&self, name: &str) -> Result<Option<KeycloakRole>, KeycloakAdminError> {
        match self
            .send(Method::GET, self.endpoint(&["roles", name])?, None)
            .await
        {
            Ok(response) => Self::read_json(response).await.map(Some),
            Err(KeycloakAdminError::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Creates the realm role `name`; an existing role is left untouched.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn create_realm_role(
        &self,
        name: &str,
        description: &str,
    ) -> Result<(), KeycloakAdminError> {
        match self
            .send(
                Method::POST,
                self.endpoint(&["roles"])?,
                Some(&json!({ "name": name, "description": description })),
            )
            .await
        {
            Ok(_) | Err(KeycloakAdminError::AlreadyExists) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Realm roles directly mapped to user `id` (composite and client roles excluded).
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotFound`] if the user vanished, plus
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn user_realm_roles(
        &self,
        id: &str,
    ) -> Result<Vec<KeycloakRole>, KeycloakAdminError> {
        let response = self
            .send(
                Method::GET,
                self.endpoint(&["users", id, "role-mappings", "realm"])?,
                None,
            )
            .await?;
        Self::read_json(response).await
    }

    /// Maps realm `roles` to user `id`, in addition to those already mapped.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotFound`] if the user or a role vanished, plus
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`].
    pub async fn add_user_realm_roles(
        &self,
        id: &str,
        roles: &[KeycloakRole],
    ) -> Result<(), KeycloakAdminError> {
        self.send(
            Method::POST,
            self.endpoint(&["users", id, "role-mappings", "realm"])?,
            Some(&json!(roles)),
        )
        .await
        .map(|_| ())
    }

    /// Asks Keycloak to e-mail user `id` a link to set their password (`UPDATE_PASSWORD`
    /// required action). Requires the realm's SMTP settings.
    ///
    /// # Errors
    ///
    /// [`KeycloakAdminError::NotFound`] if the user vanished, plus
    /// [`KeycloakAdminError::NotConfigured`], [`KeycloakAdminError::Forbidden`] or
    /// [`KeycloakAdminError::Unavailable`] (also when the realm cannot send e-mails).
    pub async fn send_password_setup_email(&self, id: &str) -> Result<(), KeycloakAdminError> {
        self.send(
            Method::PUT,
            self.endpoint(&["users", id, "execute-actions-email"])?,
            Some(&json!(["UPDATE_PASSWORD"])),
        )
        .await
        .map(|_| ())
    }
}
