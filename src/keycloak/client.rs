use super::config::KeycloakConfig;
use super::error::KeycloakError;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::RwLock;

/// Asymmetric algorithms accepted for Keycloak ID tokens. HMAC is refused so that a token can
/// never be verified with a shared secret published in the realm's key set.
const ACCEPTED_ALGORITHMS: [Algorithm; 9] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::EdDSA,
];

/// Clock skew tolerated between Keycloak and Core when checking `exp`, in seconds.
const CLOCK_SKEW_LEEWAY: u64 = 30;

/// Timeout of every HTTP call to Keycloak.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Authorization code obtained by the front from Keycloak's login page, with what is needed to
/// redeem it.
#[derive(Debug, Clone)]
pub struct AuthorizationCode<'a> {
    /// Code returned by Keycloak on the redirect URI.
    pub code: &'a str,
    /// Redirect URI sent in the authorization request; Keycloak requires the exact same value.
    pub redirect_uri: &'a str,
    /// PKCE verifier, when the authorization request carried a `code_challenge`.
    pub code_verifier: Option<&'a str>,
    /// Nonce sent in the authorization request, checked against the ID token's `nonce` claim.
    pub nonce: Option<&'a str>,
}

/// Identity asserted by a verified Keycloak ID token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeycloakIdentity {
    /// Keycloak user id (`sub` claim).
    pub subject: String,
    /// Verified e-mail address, used to find the matching Mairie 360 account.
    pub email: String,
}

/// Claims read from a Keycloak ID token; `iss`, `aud` and `exp` are checked by `jsonwebtoken`.
#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: String,
    email: Option<String>,
    #[serde(default)]
    email_verified: bool,
    nonce: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    id_token: Option<String>,
}

/// OIDC client of the Keycloak realm: redeems authorization codes and verifies ID tokens
/// against the realm's published keys, which are cached and refreshed on key rotation.
pub struct KeycloakClient {
    config: KeycloakConfig,
    http: reqwest::Client,
    jwks: RwLock<Option<JwkSet>>,
}

impl KeycloakClient {
    /// # Panics
    ///
    /// Panics if the HTTP client cannot be built (TLS backend initialisation failure), which
    /// only happens on a broken host and must stop the API at startup.
    #[must_use]
    pub fn new(config: KeycloakConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .expect("Failed to build the Keycloak HTTP client");
        Self {
            config,
            http,
            jwks: RwLock::new(None),
        }
    }

    #[must_use]
    pub const fn config(&self) -> &KeycloakConfig {
        &self.config
    }

    /// Redeems `code` at the realm's token endpoint and returns the identity carried by the
    /// verified ID token.
    ///
    /// # Errors
    ///
    /// - [`KeycloakError::InvalidGrant`] if Keycloak refuses the code;
    /// - [`KeycloakError::InvalidIdToken`] if the response holds no valid ID token;
    /// - [`KeycloakError::EmailNotVerified`] if the token has no verified e-mail;
    /// - [`KeycloakError::Unavailable`] if Keycloak cannot be reached or rejects Core's client.
    pub async fn authenticate(
        &self,
        code: &AuthorizationCode<'_>,
    ) -> Result<KeycloakIdentity, KeycloakError> {
        let id_token = self.exchange_code(code).await?;
        self.verify_id_token(&id_token, code.nonce).await
    }

    async fn exchange_code(&self, code: &AuthorizationCode<'_>) -> Result<String, KeycloakError> {
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code.code),
            ("redirect_uri", code.redirect_uri),
            ("client_id", self.config.client_id()),
        ];
        if let Some(secret) = self.config.client_secret() {
            form.push(("client_secret", secret));
        }
        if let Some(verifier) = code.code_verifier {
            form.push(("code_verifier", verifier));
        }

        let response = self
            .http
            .post(self.config.token_endpoint())
            .form(&form)
            .send()
            .await
            .map_err(|e| {
                eprintln!("Keycloak token endpoint unreachable: {e}");
                KeycloakError::Unavailable
            })?;

        let status = response.status();
        // Keycloak answers 400 `invalid_grant` for a bad, expired or reused code, and 401
        // `invalid_client` when Core's own credentials are wrong: only the former is the caller's fault.
        if status == reqwest::StatusCode::BAD_REQUEST {
            return Err(KeycloakError::InvalidGrant);
        }
        if !status.is_success() {
            eprintln!("Keycloak token endpoint answered {status}");
            return Err(KeycloakError::Unavailable);
        }

        let body: TokenResponse = response.json().await.map_err(|e| {
            eprintln!("Unreadable Keycloak token response: {e}");
            KeycloakError::Unavailable
        })?;
        // No ID token means the authorization request was sent without the `openid` scope.
        body.id_token.ok_or(KeycloakError::InvalidIdToken)
    }

    /// Verifies a Keycloak ID token: signature against the realm keys, `iss`, `aud` (must list
    /// Core's client id), `exp`, `nonce` when one is expected, and a verified e-mail.
    ///
    /// # Errors
    ///
    /// - [`KeycloakError::InvalidIdToken`] if any check on the token itself fails;
    /// - [`KeycloakError::EmailNotVerified`] if the token has no verified e-mail;
    /// - [`KeycloakError::Unavailable`] if the realm keys cannot be fetched.
    pub async fn verify_id_token(
        &self,
        id_token: &str,
        expected_nonce: Option<&str>,
    ) -> Result<KeycloakIdentity, KeycloakError> {
        let header = decode_header(id_token).map_err(|_| KeycloakError::InvalidIdToken)?;
        if !ACCEPTED_ALGORITHMS.contains(&header.alg) {
            return Err(KeycloakError::InvalidIdToken);
        }
        let kid = header.kid.ok_or(KeycloakError::InvalidIdToken)?;
        let key = self.decoding_key(&kid).await?;
        if header.alg.family() != key.family() {
            return Err(KeycloakError::InvalidIdToken);
        }

        let mut validation = Validation::new(header.alg);
        validation.leeway = CLOCK_SKEW_LEEWAY;
        validation.set_issuer(&[self.config.issuer()]);
        validation.set_audience(&[self.config.client_id()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

        let claims = decode::<IdTokenClaims>(id_token, &key, &validation)
            .map_err(|e| {
                eprintln!("Keycloak ID token rejected: {e}");
                KeycloakError::InvalidIdToken
            })?
            .claims;

        if let Some(expected) = expected_nonce {
            if claims.nonce.as_deref() != Some(expected) {
                return Err(KeycloakError::InvalidIdToken);
            }
        }

        match claims.email {
            Some(email) if claims.email_verified && !email.trim().is_empty() => {
                Ok(KeycloakIdentity {
                    subject: claims.sub,
                    email: email.trim().to_string(),
                })
            }
            _ => Err(KeycloakError::EmailNotVerified),
        }
    }

    /// Returns the realm key `kid`, fetching the key set again once if it is unknown (Keycloak
    /// rotated its keys since the last fetch).
    async fn decoding_key(&self, kid: &str) -> Result<DecodingKey, KeycloakError> {
        if let Some(key) = self.cached_key(kid).await {
            return key;
        }
        let jwks = self.fetch_jwks().await?;
        let key = jwks.find(kid).map(DecodingKey::from_jwk);
        *self.jwks.write().await = Some(jwks);
        key.map_or(Err(KeycloakError::InvalidIdToken), |key| {
            key.map_err(|_| KeycloakError::InvalidIdToken)
        })
    }

    async fn cached_key(&self, kid: &str) -> Option<Result<DecodingKey, KeycloakError>> {
        let jwks = self.jwks.read().await;
        jwks.as_ref()?
            .find(kid)
            .map(|jwk| DecodingKey::from_jwk(jwk).map_err(|_| KeycloakError::InvalidIdToken))
    }

    async fn fetch_jwks(&self) -> Result<JwkSet, KeycloakError> {
        let response = self
            .http
            .get(self.config.jwks_uri())
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| {
                eprintln!("Keycloak key set unreachable: {e}");
                KeycloakError::Unavailable
            })?;
        response.json().await.map_err(|e| {
            eprintln!("Unreadable Keycloak key set: {e}");
            KeycloakError::Unavailable
        })
    }
}
