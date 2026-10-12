use super::view::{PasskeyLoginOptionsResponseView, PasskeyLoginView};
use crate::client_ip::client_ip;
use crate::database::ids::id_from_sql;
use crate::database::passkeys::credential_id_hex;
use crate::database::passkeys::find_by_credential::{
    FindPasskeyByCredentialQueryView, StoredPasskey,
};
use crate::database::passkeys::update_credential::UpdatePasskeyCredentialQueryView;
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::v1::auth::login::endpoint::{generate_session, LoginError};
use crate::endpoints::v1::auth::login::view::LoginResponseView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::webauthn::{user_handle, ChallengeStore, PendingAuthentication};
use actix_web::{http::StatusCode, post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;
use webauthn_rs::prelude::{DiscoverableKey, Passkey, Webauthn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasskeyLoginError {
    /// Unknown, expired or already used challenge, unknown credential, assertion refused by the
    /// `WebAuthn` verification, archived account: one `401`, the cases are not told apart.
    AuthenticationFailed,
    DatabaseError,
    NotConfigured,
    RedisError,
    TokenGenerationError,
    TooManyRequests(u64),
}

impl std::fmt::Display for PasskeyLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthenticationFailed => write!(f, "Passkey authentication failed."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
            Self::NotConfigured => write!(f, "Passkey sign-in is not configured."),
            Self::RedisError => write!(f, "Internal Redis error."),
            Self::TokenGenerationError => write!(f, "Failed to generate JWT token."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
        }
    }
}

impl ResponseError for PasskeyLoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::AuthenticationFailed => StatusCode::UNAUTHORIZED,
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Self::DatabaseError | Self::RedisError | Self::TokenGenerationError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

/// Counts the attempt against the per-address budget shared with the password login.
fn check_rate_limit(
    limits: Option<&RateLimits>,
    ip_address: std::net::IpAddr,
) -> Result<(), PasskeyLoginError> {
    limits.map_or(Ok(()), |limits| {
        limits
            .login_per_ip
            .hit(&ip_key(ip_address))
            .map_err(|refused| PasskeyLoginError::TooManyRequests(refused.retry_after_seconds))
    })
}

#[utoipa::path(
    post,
    path = "/options",
    summary = "Start a passkey sign-in",
    description = "Opens a passkey (WebAuthn) sign-in ceremony and returns the options to hand to \
                   `navigator.credentials.get()`. No e-mail is sent: the credential is \
                   **discoverable**, the browser shows the passkeys registered for this site \
                   (conditional UI or modal), so nothing tells which addresses have an account.\n\n\
                   The ceremony is identified by `challenge_id`, to send back with the assertion to \
                   `POST /api/v1/auth/passkey`. It is single use and expires after two minutes.\n\n\
                   Public route (the `JwtMiddleware` lets everything under `/auth` through). Rate \
                   limited per client address with the password login (30 attempts per minute).",
    responses(
        (
            status = 200,
            description = "Ceremony opened: the request options and the id of the ceremony.",
            body = PasskeyLoginOptionsResponseView,
            example = json!({
                "challenge_id": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
                "public_key": {
                    "publicKey": {
                        "challenge": "xLqpXkT3oF6Q2oFqaX0S5nHyR8yq9b2N7xG4w0a5r1M",
                        "timeout": 60000,
                        "rpId": "mairie360.fr",
                        "allowCredentials": [],
                        "userVerification": "required"
                    }
                }
            })
        ),
        (
            status = 429,
            description = "Too many attempts from this client address (shared with `POST /api/v1/auth/login`). `Retry-After` gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(
                ("Retry-After" = u64, description = "Seconds to wait before the next attempt.")
            ),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Internal error: the ceremony could not be opened or stored in Redis.",
            body = String,
            content_type = "text/plain",
            example = json!("Internal Redis error.")
        ),
        (
            status = 503,
            description = "Passkeys are disabled on this instance (`WEBAUTHN_RP_ID` or `WEBAUTHN_RP_ORIGIN` not set). Use `POST /api/v1/auth/login` instead.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkey sign-in is not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/passkey/options")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn passkey_login_options(
    state: web::Data<AppState>,
    webauthn: Option<web::Data<Webauthn>>,
    limits: Option<web::Data<RateLimits>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeyLoginError> {
    let webauthn = webauthn.ok_or(PasskeyLoginError::NotConfigured)?;
    check_rate_limit(limits.as_ref().map(web::Data::get_ref), client_ip(&request))?;

    let (public_key, state_) = webauthn.start_discoverable_authentication().map_err(|e| {
        tracing::error!("Passkey sign-in: cannot open the ceremony: {e}");
        PasskeyLoginError::RedisError
    })?;
    let challenge_id = ChallengeStore::new(state.get_redis())
        .store_authentication(&PendingAuthentication { state: state_ })
        .await
        .map_err(|e| {
            tracing::error!("Passkey sign-in: cannot store the ceremony: {e}");
            PasskeyLoginError::RedisError
        })?;

    Ok(
        HttpResponse::Ok().json(PasskeyLoginOptionsResponseView::new(
            challenge_id,
            public_key,
        )),
    )
}

/// Checks the assertion against the pending ceremony and the stored passkey, updates the
/// passkey (counter, flags, last use) and returns the id of the account signing in.
async fn authenticate(
    login_view: &PasskeyLoginView,
    webauthn: &Webauthn,
    state: &web::Data<AppState>,
) -> Result<u64, PasskeyLoginError> {
    let pending = ChallengeStore::new(state.get_redis())
        .take_authentication(login_view.challenge_id())
        .await
        .map_err(|e| {
            tracing::error!("Passkey sign-in: cannot read the ceremony: {e}");
            PasskeyLoginError::RedisError
        })?
        .ok_or_else(|| {
            tracing::warn!("Passkey sign-in refused: unknown, expired or replayed challenge");
            PasskeyLoginError::AuthenticationFailed
        })?;

    let credential = login_view.credential();
    let stored: StoredPasskey = match state
        .get_smart_db()
        .fetch_one(&FindPasskeyByCredentialQueryView::new(&credential_id_hex(
            credential.get_credential_id(),
        )))
        .await
    {
        Ok(stored) => stored,
        Err(e) => {
            return Err(
                match db_error::log("passkey sign-in: find credential", &e) {
                    DbFailure::NotFound => {
                        tracing::warn!("Passkey sign-in refused: unknown credential");
                        PasskeyLoginError::AuthenticationFailed
                    }
                    _ => PasskeyLoginError::DatabaseError,
                },
            );
        }
    };
    let user_id = id_from_sql(stored.user_id);

    // A discoverable assertion carries the user handle the authenticator stored at
    // registration: when present it must be the one of the account owning the credential.
    if credential
        .get_user_unique_id()
        .is_some_and(|handle| handle != user_handle(user_id).as_bytes())
    {
        tracing::warn!("Passkey sign-in refused: user handle does not match the credential");
        return Err(PasskeyLoginError::AuthenticationFailed);
    }

    let mut passkey: Passkey = serde_json::from_value(stored.passkey).map_err(|e| {
        tracing::error!(
            "Passkey sign-in: stored credential {} is unreadable: {e}",
            stored.id
        );
        PasskeyLoginError::DatabaseError
    })?;
    let result = webauthn
        .finish_discoverable_authentication(
            credential,
            pending.state,
            &[DiscoverableKey::from(&passkey)],
        )
        .map_err(|e| {
            tracing::warn!("Passkey sign-in refused: {e}");
            PasskeyLoginError::AuthenticationFailed
        })?;

    // Counter and backup flags move rarely, `last_used_at` on every sign-in. Done before the
    // archived check so a cloned-authenticator counter is recorded whatever the account state.
    passkey.update_credential(&result);
    let serialised = serde_json::to_string(&passkey).map_err(|e| {
        tracing::error!(
            "Passkey sign-in: cannot serialise credential {}: {e}",
            stored.id
        );
        PasskeyLoginError::DatabaseError
    })?;
    state
        .get_smart_db()
        .execute(UpdatePasskeyCredentialQueryView::new(
            stored.id,
            &serialised,
        ))
        .await
        .map_err(|e| {
            db_error::log("passkey sign-in: update credential", &e);
            PasskeyLoginError::DatabaseError
        })?;

    if stored.is_archived {
        tracing::warn!("Passkey sign-in refused: archived account {user_id}");
        return Err(PasskeyLoginError::AuthenticationFailed);
    }
    Ok(user_id)
}

#[utoipa::path(
    post,
    path = "",
    summary = "Sign in with a passkey",
    description = "Completes the passkey (WebAuthn) sign-in opened by `POST \
                   /api/v1/auth/passkey/options`: checks the assertion (origin, relying party, \
                   challenge, signature, user verification, signature counter) against the passkey \
                   registered under its credential id, then opens a session for the account owning \
                   it.\n\n\
                   The response is the same as `POST /api/v1/auth/login`: a Core JWT in the \
                   `Authorization` header and a refresh token in the body. The first-connection \
                   password change does not apply: registering a passkey already required a \
                   session.\n\n\
                   An unknown, expired or replayed `challenge_id`, an unknown credential, a refused \
                   assertion and an archived account all answer the same `401`. Public route, rate \
                   limited per client address with the password login.",
    request_body(
        content = PasskeyLoginView,
        description = "The ceremony id, the assertion as serialised by `PublicKeyCredential.toJSON()` and a description of the device.",
        example = json!({
            "challenge_id": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
            "credential": {
                "id": "vJd5R8m2oA7N_3kQ1eF2hWfYbZcTx9L0",
                "rawId": "vJd5R8m2oA7N_3kQ1eF2hWfYbZcTx9L0",
                "type": "public-key",
                "response": {
                    "authenticatorData": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFAAAAAQ",
                    "clientDataJSON": "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoi...",
                    "signature": "MEUCIQDx...",
                    "userHandle": "m3q0Nq1xSfe2x4y7q0cNhA"
                },
                "clientExtensionResults": {}
            },
            "device_info": "Safari 26 on iPhone"
        })
    ),
    responses(
        (
            status = 200,
            description = "Signed in. The JWT is in the `Authorization` header, the refresh token in the body.",
            body = LoginResponseView,
            headers(
                ("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")
            ),
            example = json!({ "refresh_token": "refresh-token-of-the-session" })
        ),
        (
            status = 400,
            description = "Malformed body: missing `challenge_id`, `credential` or `device_info`, a credential that is not a WebAuthn `PublicKeyCredential`, or `device_info` longer than 512 characters or holding a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Json deserialize error: missing field `credential` at line 1 column 42")
        ),
        (
            status = 401,
            description = "Unknown, expired or already used `challenge_id`, unknown credential, assertion refused (origin, challenge, signature, user verification, counter) or archived account. The cases are not told apart; start the sign-in again.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkey authentication failed.")
        ),
        (
            status = 429,
            description = "Too many attempts from this client address (shared with `POST /api/v1/auth/login`). `Retry-After` gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(
                ("Retry-After" = u64, description = "Seconds to wait before the next attempt.")
            ),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Internal error: database, Redis or JWT generation.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "Passkeys are disabled on this instance (`WEBAUTHN_RP_ID` or `WEBAUTHN_RP_ORIGIN` not set). Use `POST /api/v1/auth/login` instead.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkey sign-in is not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/passkey")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn passkey_login(
    payload: ValidatedJson<PasskeyLoginView>,
    state: web::Data<AppState>,
    webauthn: Option<web::Data<Webauthn>>,
    limits: Option<web::Data<RateLimits>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeyLoginError> {
    let webauthn = webauthn.ok_or(PasskeyLoginError::NotConfigured)?;
    let login_view = payload.into_inner();
    let ip_address = client_ip(&request);
    check_rate_limit(limits.as_ref().map(web::Data::get_ref), ip_address)?;

    let user_id = authenticate(&login_view, &webauthn, &state).await?;

    let (jwt, refresh_token) =
        generate_session(user_id, login_view.device_info(), ip_address, state)
            .await
            .map_err(|e| match e {
                LoginError::TokenGenerationError => PasskeyLoginError::TokenGenerationError,
                _ => PasskeyLoginError::DatabaseError,
            })?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token)))
}
