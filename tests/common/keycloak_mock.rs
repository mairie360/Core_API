//! Minimal Keycloak realm for tests: an OIDC token endpoint, a JWKS endpoint and the subset of
//! the Admin REST API the account migration uses, served by a real HTTP server on a random
//! local port, with ID tokens signed by throwaway RSA keys.

// Fake-realm handlers: actix handlers are `!Send` (they hold an `HttpRequest`), the realm state
// is a plain mutex held for the whole synchronous handler body, and the early-return
// `HttpResponse` of the authorization guard is large by nature.
#![allow(
    clippy::future_not_send,
    clippy::significant_drop_tightening,
    clippy::result_large_err
)]

use actix_web::{http::StatusCode, web, App, HttpRequest, HttpResponse, HttpServer};
use core_api::keycloak::{KeycloakAdminClient, KeycloakClient, KeycloakConfig};
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

const REALM_PATH: &str = "/realms/test";
const ADMIN_PATH: &str = "/admin/realms/test";

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

/// Keycloak user id (`sub`) the fake realm assigns to `email`: stable per address, ignoring
/// case, and distinct between addresses, so tests sharing the database never collide on a
/// subject.
pub fn subject_for(email: &str) -> String {
    let slug: String = email
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("sub-{slug}")
}

/// Claims of a valid ID token for `email`, issued by `issuer` to [`CLIENT_ID`].
pub fn id_token_claims(issuer: &str, email: &str) -> Value {
    let now = get_current_timestamp();
    json!({
        "iss": issuer,
        "aud": CLIENT_ID,
        "sub": subject_for(email),
        "azp": CLIENT_ID,
        "iat": now,
        "exp": now + 300,
        "email": email,
        "email_verified": true,
        "nonce": "test-nonce"
    })
}

/// A user of the fake realm, as the Admin API stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub enabled: bool,
    pub email_verified: bool,
    /// Names of the realm roles mapped to the user.
    pub realm_roles: Vec<String>,
}

impl MockUser {
    fn representation(&self) -> Value {
        json!({
            "id": self.id,
            "username": self.username,
            "email": self.email,
            "firstName": self.first_name,
            "lastName": self.last_name,
            "enabled": self.enabled,
            "emailVerified": self.email_verified
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockRole {
    pub id: String,
    pub name: String,
}

impl MockRole {
    fn representation(&self) -> Value {
        json!({ "id": self.id, "name": self.name })
    }
}

/// Admin side of the fake realm.
#[derive(Default)]
struct AdminRealm {
    users: Vec<MockUser>,
    roles: Vec<MockRole>,
    /// Ids of the users Keycloak was asked to e-mail a password set-up link.
    password_emails: Vec<String>,
    /// Service-account token currently accepted by the Admin API (`None`: every token expired).
    valid_token: Option<String>,
    issued_tokens: usize,
    /// The token endpoint refuses the `client_credentials` grant.
    deny_service_account: bool,
    /// Every Admin API call answers `403`.
    forbid_admin_api: bool,
    /// `execute-actions-email` answers `500` (realm without SMTP).
    fail_password_emails: bool,
}

struct MockState {
    jwks: Mutex<Value>,
    token_response: Mutex<(u16, Value)>,
    token_forms: Mutex<Vec<HashMap<String, String>>>,
    jwks_hits: AtomicUsize,
    admin: Mutex<AdminRealm>,
}

/// A running fake Keycloak realm. The server stops when the test's runtime shuts down.
pub struct KeycloakMock {
    realm_url: String,
    state: Arc<MockState>,
}

impl KeycloakMock {
    /// Starts, on the current Tokio runtime, a realm publishing [`TestKey::A`] only, whose
    /// token endpoint answers `500` to authorization codes until
    /// [`KeycloakMock::respond_with_id_token`] or [`KeycloakMock::respond_with`] is called, and
    /// whose Admin API starts empty.
    pub fn start() -> Self {
        let state = Arc::new(MockState {
            jwks: Mutex::new(json!({ "keys": [TestKey::A.jwk()] })),
            token_response: Mutex::new((500, json!({ "error": "not_configured" }))),
            token_forms: Mutex::new(Vec::new()),
            jwks_hits: AtomicUsize::new(0),
            admin: Mutex::new(AdminRealm::default()),
        });
        let data = web::Data::from(state.clone());
        let server = HttpServer::new(move || {
            App::new()
                .app_data(data.clone())
                .route(
                    &format!("{REALM_PATH}/protocol/openid-connect/token"),
                    web::post().to(token),
                )
                .route(
                    &format!("{REALM_PATH}/protocol/openid-connect/certs"),
                    web::get().to(certs),
                )
                .service(
                    web::scope(ADMIN_PATH)
                        .route("/users", web::get().to(admin_list_users))
                        .route("/users", web::post().to(admin_create_user))
                        .route("/users/{id}", web::get().to(admin_get_user))
                        .route("/users/{id}", web::put().to(admin_update_user))
                        .route(
                            "/users/{id}/execute-actions-email",
                            web::put().to(admin_execute_actions_email),
                        )
                        .route(
                            "/users/{id}/role-mappings/realm",
                            web::get().to(admin_user_realm_roles),
                        )
                        .route(
                            "/users/{id}/role-mappings/realm",
                            web::post().to(admin_add_user_realm_roles),
                        )
                        .route("/roles", web::post().to(admin_create_role))
                        .route("/roles/{name}", web::get().to(admin_get_role)),
                )
        })
        .workers(1)
        .disable_signals()
        .bind(("127.0.0.1", 0))
        .expect("bind Keycloak mock");
        let port = server.addrs()[0].port();
        tokio::spawn(server.run());
        Self {
            realm_url: format!("http://127.0.0.1:{port}{REALM_PATH}"),
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

    /// Configuration of a public client (no secret) pointing at this realm.
    pub fn public_config(&self) -> KeycloakConfig {
        KeycloakConfig::new(&self.realm_url, None, CLIENT_ID, None)
    }

    pub fn client(&self) -> KeycloakClient {
        KeycloakClient::new(self.config())
    }

    pub fn admin_client(&self) -> KeycloakAdminClient {
        KeycloakAdminClient::new(self.config())
    }

    /// Makes the token endpoint answer `status` with `body` to authorization codes.
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

    /// Form bodies received by the token endpoint, oldest first (all grant types).
    pub fn token_requests(&self) -> Vec<HashMap<String, String>> {
        self.state.token_forms.lock().unwrap().clone()
    }

    // --- Admin API -------------------------------------------------------------------------

    /// Number of service-account tokens issued (`client_credentials` grants accepted).
    pub fn admin_token_grants(&self) -> usize {
        self.state.admin.lock().unwrap().issued_tokens
    }

    /// Makes the token endpoint refuse the `client_credentials` grant (`401 invalid_client`).
    pub fn deny_service_account(&self) {
        self.state.admin.lock().unwrap().deny_service_account = true;
    }

    /// Makes every Admin API call answer `403`.
    pub fn forbid_admin_api(&self) {
        self.state.admin.lock().unwrap().forbid_admin_api = true;
    }

    /// Invalidates the service-account token in use: the next Admin API call answers `401`
    /// until a new token is requested.
    pub fn expire_admin_token(&self) {
        self.state.admin.lock().unwrap().valid_token = None;
    }

    /// Makes `execute-actions-email` fail (`500`), as a realm without SMTP settings does.
    pub fn fail_password_emails(&self) {
        self.state.admin.lock().unwrap().fail_password_emails = true;
    }

    /// Adds a user to the realm, as if created by hand, and returns its Keycloak id.
    pub fn seed_user(&self, email: &str, enabled: bool) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.state.admin.lock().unwrap().users.push(MockUser {
            id: id.clone(),
            username: email.to_lowercase(),
            email: email.to_lowercase(),
            first_name: "Seeded".to_string(),
            last_name: "ByHand".to_string(),
            enabled,
            email_verified: false,
            realm_roles: Vec::new(),
        });
        id
    }

    /// Adds a realm role and returns its id.
    pub fn seed_role(&self, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.state.admin.lock().unwrap().roles.push(MockRole {
            id: id.clone(),
            name: name.to_string(),
        });
        id
    }

    /// Removes a user, as a realm rebuilt from scratch would.
    pub fn delete_user(&self, id: &str) {
        self.state
            .admin
            .lock()
            .unwrap()
            .users
            .retain(|user| user.id != id);
    }

    pub fn users(&self) -> Vec<MockUser> {
        self.state.admin.lock().unwrap().users.clone()
    }

    pub fn user(&self, id: &str) -> Option<MockUser> {
        self.users().into_iter().find(|user| user.id == id)
    }

    pub fn user_by_email(&self, email: &str) -> Option<MockUser> {
        self.users()
            .into_iter()
            .find(|user| user.email.eq_ignore_ascii_case(email))
    }

    pub fn roles(&self) -> Vec<MockRole> {
        self.state.admin.lock().unwrap().roles.clone()
    }

    /// Ids of the users Keycloak was asked to e-mail a password set-up link, oldest first.
    pub fn password_emails(&self) -> Vec<String> {
        self.state.admin.lock().unwrap().password_emails.clone()
    }
}

async fn token(
    state: web::Data<MockState>,
    form: web::Form<HashMap<String, String>>,
) -> HttpResponse {
    let form = form.into_inner();
    state.token_forms.lock().unwrap().push(form.clone());
    if form.get("grant_type").map(String::as_str) == Some("client_credentials") {
        let mut admin = state.admin.lock().unwrap();
        let credentials_ok = form.get("client_id").map(String::as_str) == Some(CLIENT_ID)
            && form.get("client_secret").map(String::as_str) == Some(CLIENT_SECRET);
        if admin.deny_service_account || !credentials_ok {
            return HttpResponse::Unauthorized().json(json!({ "error": "invalid_client" }));
        }
        admin.issued_tokens += 1;
        let token = format!("admin-token-{}", admin.issued_tokens);
        admin.valid_token = Some(token.clone());
        return HttpResponse::Ok().json(json!({
            "access_token": token,
            "expires_in": 60,
            "token_type": "Bearer",
            "scope": "profile email"
        }));
    }
    let (status, body) = state.token_response.lock().unwrap().clone();
    HttpResponse::build(StatusCode::from_u16(status).unwrap()).json(body)
}

async fn certs(state: web::Data<MockState>) -> HttpResponse {
    state.jwks_hits.fetch_add(1, Ordering::SeqCst);
    let jwks = state.jwks.lock().unwrap().clone();
    HttpResponse::Ok().json(jwks)
}

/// `Err` with the response to send when the request must not reach the Admin API.
fn authorize(req: &HttpRequest, admin: &AdminRealm) -> Result<(), HttpResponse> {
    if admin.forbid_admin_api {
        return Err(HttpResponse::Forbidden().json(json!({ "error": "unknown_error" })));
    }
    let bearer = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match (bearer, admin.valid_token.as_deref()) {
        (Some(sent), Some(valid)) if sent == valid => Ok(()),
        _ => Err(HttpResponse::Unauthorized().json(json!({ "error": "HTTP 401 Unauthorized" }))),
    }
}

macro_rules! admin_guard {
    ($req:expr, $admin:expr) => {
        if let Err(response) = authorize(&$req, &$admin) {
            return response;
        }
    };
}

fn string_field(body: &Value, field: &str) -> Option<String> {
    body.get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

async fn admin_list_users(
    req: HttpRequest,
    state: web::Data<MockState>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    let admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    let email = query.get("email").map(|email| email.to_lowercase());
    let users: Vec<Value> = admin
        .users
        .iter()
        .filter(|user| {
            email
                .as_deref()
                .is_none_or(|email| user.email.to_lowercase() == email)
        })
        .map(MockUser::representation)
        .collect();
    HttpResponse::Ok().json(users)
}

async fn admin_create_user(
    req: HttpRequest,
    state: web::Data<MockState>,
    body: web::Json<Value>,
) -> HttpResponse {
    let mut admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    let username = string_field(&body, "username").unwrap_or_default();
    let email = string_field(&body, "email").unwrap_or_default();
    if admin.users.iter().any(|user| {
        user.username.eq_ignore_ascii_case(&username) || user.email.eq_ignore_ascii_case(&email)
    }) {
        return HttpResponse::Conflict()
            .json(json!({ "errorMessage": "User exists with same username or email" }));
    }
    let id = uuid::Uuid::new_v4().to_string();
    admin.users.push(MockUser {
        id: id.clone(),
        // Keycloak stores usernames and e-mails in lower case.
        username: username.to_lowercase(),
        email: email.to_lowercase(),
        first_name: string_field(&body, "firstName").unwrap_or_default(),
        last_name: string_field(&body, "lastName").unwrap_or_default(),
        enabled: body["enabled"].as_bool().unwrap_or(false),
        email_verified: body["emailVerified"].as_bool().unwrap_or(false),
        realm_roles: Vec::new(),
    });
    let host = req.connection_info().host().to_string();
    HttpResponse::Created()
        .insert_header(("Location", format!("http://{host}{ADMIN_PATH}/users/{id}")))
        .finish()
}

async fn admin_get_user(
    req: HttpRequest,
    state: web::Data<MockState>,
    id: web::Path<String>,
) -> HttpResponse {
    let admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    admin.users.iter().find(|user| user.id == *id).map_or_else(
        || HttpResponse::NotFound().json(json!({ "error": "User not found" })),
        |user| HttpResponse::Ok().json(user.representation()),
    )
}

async fn admin_update_user(
    req: HttpRequest,
    state: web::Data<MockState>,
    id: web::Path<String>,
    body: web::Json<Value>,
) -> HttpResponse {
    let mut admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    let Some(user) = admin.users.iter_mut().find(|user| user.id == *id) else {
        return HttpResponse::NotFound().json(json!({ "error": "User not found" }));
    };
    if let Some(email) = string_field(&body, "email") {
        user.email = email.to_lowercase();
    }
    if let Some(first_name) = string_field(&body, "firstName") {
        user.first_name = first_name;
    }
    if let Some(last_name) = string_field(&body, "lastName") {
        user.last_name = last_name;
    }
    if let Some(enabled) = body["enabled"].as_bool() {
        user.enabled = enabled;
    }
    if let Some(verified) = body["emailVerified"].as_bool() {
        user.email_verified = verified;
    }
    HttpResponse::NoContent().finish()
}

async fn admin_execute_actions_email(
    req: HttpRequest,
    state: web::Data<MockState>,
    id: web::Path<String>,
    body: web::Json<Value>,
) -> HttpResponse {
    let mut admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    if !admin.users.iter().any(|user| user.id == *id) {
        return HttpResponse::NotFound().json(json!({ "error": "User not found" }));
    }
    if admin.fail_password_emails {
        return HttpResponse::InternalServerError()
            .json(json!({ "errorMessage": "Failed to send execute actions email" }));
    }
    assert_eq!(*body, json!(["UPDATE_PASSWORD"]), "required actions");
    admin.password_emails.push(id.into_inner());
    HttpResponse::NoContent().finish()
}

async fn admin_user_realm_roles(
    req: HttpRequest,
    state: web::Data<MockState>,
    id: web::Path<String>,
) -> HttpResponse {
    let admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    let Some(user) = admin.users.iter().find(|user| user.id == *id) else {
        return HttpResponse::NotFound().json(json!({ "error": "User not found" }));
    };
    let roles: Vec<Value> = admin
        .roles
        .iter()
        .filter(|role| user.realm_roles.contains(&role.name))
        .map(MockRole::representation)
        .collect();
    HttpResponse::Ok().json(roles)
}

async fn admin_add_user_realm_roles(
    req: HttpRequest,
    state: web::Data<MockState>,
    id: web::Path<String>,
    body: web::Json<Vec<Value>>,
) -> HttpResponse {
    let mut admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    let mut names = Vec::new();
    for role in body.iter() {
        let known = admin.roles.iter().find(|known| {
            Some(known.id.as_str()) == role["id"].as_str()
                && Some(known.name.as_str()) == role["name"].as_str()
        });
        match known {
            Some(known) => names.push(known.name.clone()),
            None => return HttpResponse::NotFound().json(json!({ "error": "Role not found" })),
        }
    }
    let Some(user) = admin.users.iter_mut().find(|user| user.id == *id) else {
        return HttpResponse::NotFound().json(json!({ "error": "User not found" }));
    };
    for name in names {
        if !user.realm_roles.contains(&name) {
            user.realm_roles.push(name);
        }
    }
    HttpResponse::NoContent().finish()
}

async fn admin_create_role(
    req: HttpRequest,
    state: web::Data<MockState>,
    body: web::Json<Value>,
) -> HttpResponse {
    let mut admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    let name = string_field(&body, "name").unwrap_or_default();
    if admin.roles.iter().any(|role| role.name == name) {
        return HttpResponse::Conflict()
            .json(json!({ "errorMessage": "Role with name already exists" }));
    }
    admin.roles.push(MockRole {
        id: uuid::Uuid::new_v4().to_string(),
        name,
    });
    HttpResponse::Created().finish()
}

async fn admin_get_role(
    req: HttpRequest,
    state: web::Data<MockState>,
    name: web::Path<String>,
) -> HttpResponse {
    let admin = state.admin.lock().unwrap();
    admin_guard!(req, admin);
    admin
        .roles
        .iter()
        .find(|role| role.name == *name)
        .map_or_else(
            || HttpResponse::NotFound().json(json!({ "error": "Could not find role" })),
            |role| HttpResponse::Ok().json(role.representation()),
        )
}
