//! Minimal Keycloak realm for tests: an OIDC token endpoint and a JWKS endpoint served by a real
//! HTTP server on a random local port, with ID tokens signed by throwaway RSA keys.

use actix_web::{web, App, HttpResponse, HttpServer};
use core_api::keycloak::{KeycloakClient, KeycloakConfig};
use jsonwebtoken::{encode, get_current_timestamp, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub const CLIENT_ID: &str = "core-api";
pub const CLIENT_SECRET: &str = "test-client-secret";
pub const REDIRECT_URI: &str = "https://login.mairie360.test/auth/callback";

/// Test-only RSA keys (PKCS#1 DER), never used outside this test suite.
const KEY_A_DER: &[u8] = include_bytes!("../fixtures/keycloak_test_key_a.der");
const KEY_B_DER: &[u8] = include_bytes!("../fixtures/keycloak_test_key_b.der");
const KEY_A_MODULUS: &str = "s9ZCC3V_xPhis5I_YxNyI82_KXHx5tJQz3iPms37cgHcJIJCaf7VLnLdNWx983Fhxg1rr1kibZGHzKiaZ5i9lWxR6XxXglNJjyezEGIl70mjno5AjdK7Fl5yXtF1ZyJWxMYHH5y-dm8xXWT91EuavP0_ev7HiFbgyTZB4S56UBuyhHI3O7iDT62SwW1LEuCBvJEU7bimU9k45J5zOqgxMg6BMMDRiL-snjl3nkH673_x0kyomeVPi1F0AXYPitfDrY-Du0fMDaDkyzpRNoDZtE0Z_-Dm2RiYju4MH0A-W3P8swIixX-jN6-WwFr2EnL28SsdsIPxpXJqyElK5NGjVw";
const KEY_B_MODULUS: &str = "uOIjBQaFmTXWSNft3hVbQga-aJfUwDljpt0MYuqmblYb38ennFqEi8x16Bdml12_k0SiwNANyW6b7MQm07F0wXov59zxUGizgHf7ClCSPu5Gj66117N_LSu09sgOz2hJwqycW6CAKUomuAQrAqoAzNLJHmgm3g3vNWV7nGo_yGsa6OjKWMjaycuawBgaEuj9NHAHQbrmDJ7-P82HApB3YUfiDBn9ICG2calkUFis627s8DJFS-FM-TPSWuffFiB-41Khk_jjJ1qWN1I4r3Lkw8H55_AtbCu_2TqJV4qi-WMM0X9fxW_DKmG4nOWI6sJDQiQtiozIv7VpUQ0_t8Jgkw";

/// One of the two test signing keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKey {
    A,
    B,
}

impl TestKey {
    pub const fn kid(self) -> &'static str {
        match self {
            Self::A => "test-key-a",
            Self::B => "test-key-b",
        }
    }

    const fn der(self) -> &'static [u8] {
        match self {
            Self::A => KEY_A_DER,
            Self::B => KEY_B_DER,
        }
    }

    const fn modulus(self) -> &'static str {
        match self {
            Self::A => KEY_A_MODULUS,
            Self::B => KEY_B_MODULUS,
        }
    }

    /// Public JWK of this key, as Keycloak publishes it on its certs endpoint.
    pub fn jwk(self) -> Value {
        json!({
            "kid": self.kid(),
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "n": self.modulus(),
            "e": "AQAB"
        })
    }
}

/// Signs `claims` with `key`, announcing `kid` in the header (normally `key.kid()`).
pub fn sign(claims: &Value, key: TestKey, kid: &str) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_string());
    encode(&header, claims, &EncodingKey::from_rsa_der(key.der())).expect("sign test ID token")
}

/// Claims of a valid ID token for `email`, issued by `issuer` to [`CLIENT_ID`].
pub fn id_token_claims(issuer: &str, email: &str) -> Value {
    let now = get_current_timestamp();
    json!({
        "iss": issuer,
        "aud": CLIENT_ID,
        "sub": "f3b2c1d0-7a6e-4c5b-9d8e-1a2b3c4d5e6f",
        "azp": CLIENT_ID,
        "iat": now,
        "exp": now + 300,
        "email": email,
        "email_verified": true,
        "nonce": "test-nonce"
    })
}

struct MockState {
    jwks: Mutex<Value>,
    token_response: Mutex<(u16, Value)>,
    token_forms: Mutex<Vec<HashMap<String, String>>>,
    jwks_hits: AtomicUsize,
}

/// A running fake Keycloak realm. The server stops when the test's runtime shuts down.
pub struct KeycloakMock {
    realm_url: String,
    state: Arc<MockState>,
}

impl KeycloakMock {
    /// Starts, on the current Tokio runtime, a realm publishing [`TestKey::A`] only, whose
    /// token endpoint answers `500` until [`KeycloakMock::respond_with_id_token`] or
    /// [`KeycloakMock::respond_with`] is called.
    pub fn start() -> Self {
        let state = Arc::new(MockState {
            jwks: Mutex::new(json!({ "keys": [TestKey::A.jwk()] })),
            token_response: Mutex::new((500, json!({ "error": "not_configured" }))),
            token_forms: Mutex::new(Vec::new()),
            jwks_hits: AtomicUsize::new(0),
        });
        let data = web::Data::from(state.clone());
        let server = HttpServer::new(move || {
            App::new()
                .app_data(data.clone())
                .route(
                    "/realms/test/protocol/openid-connect/token",
                    web::post().to(token),
                )
                .route(
                    "/realms/test/protocol/openid-connect/certs",
                    web::get().to(certs),
                )
        })
        .workers(1)
        .disable_signals()
        .bind(("127.0.0.1", 0))
        .expect("bind Keycloak mock");
        let port = server.addrs()[0].port();
        tokio::spawn(server.run());
        Self {
            realm_url: format!("http://127.0.0.1:{port}/realms/test"),
            state,
        }
    }

    /// Realm URL, also the issuer of the tokens it signs.
    pub fn realm_url(&self) -> &str {
        &self.realm_url
    }

    /// Configuration of a confidential client pointing at this realm.
    pub fn config(&self) -> KeycloakConfig {
        KeycloakConfig::new(&self.realm_url, None, CLIENT_ID, Some(CLIENT_SECRET))
    }

    pub fn client(&self) -> KeycloakClient {
        KeycloakClient::new(self.config())
    }

    /// Makes the token endpoint answer `status` with `body`.
    pub fn respond_with(&self, status: u16, body: Value) {
        *self.state.token_response.lock().unwrap() = (status, body);
    }

    /// Makes the token endpoint return `id_token` as a successful code exchange.
    pub fn respond_with_id_token(&self, id_token: &str) {
        self.respond_with(
            200,
            json!({
                "access_token": "opaque-access-token",
                "expires_in": 300,
                "refresh_expires_in": 1800,
                "refresh_token": "opaque-refresh-token",
                "token_type": "Bearer",
                "id_token": id_token,
                "scope": "openid email profile"
            }),
        );
    }

    /// Replaces the published key set.
    pub fn publish_keys(&self, keys: &[TestKey]) {
        let keys: Vec<Value> = keys.iter().map(|key| key.jwk()).collect();
        *self.state.jwks.lock().unwrap() = json!({ "keys": keys });
    }

    /// Number of times the key set was downloaded.
    pub fn jwks_hits(&self) -> usize {
        self.state.jwks_hits.load(Ordering::SeqCst)
    }

    /// Form bodies received by the token endpoint, oldest first.
    pub fn token_requests(&self) -> Vec<HashMap<String, String>> {
        self.state.token_forms.lock().unwrap().clone()
    }
}

async fn token(
    state: web::Data<MockState>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    state.token_forms.lock().unwrap().push(form.into_inner());
    let (status, body) = state.token_response.lock().unwrap().clone();
    HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap()).json(body)
}

async fn certs(state: web::Data<MockState>) -> HttpResponse {
    state.jwks_hits.fetch_add(1, Ordering::SeqCst);
    let jwks = state.jwks.lock().unwrap().clone();
    HttpResponse::Ok().json(jwks)
}
