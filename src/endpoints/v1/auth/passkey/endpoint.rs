use super::view::{PasskeyLoginOptionsView, PasskeyLoginView};
use crate::client_ip::client_ip;
use crate::database::passkeys::{FindPasskeyQueryView, PasskeyOwnerRecord, UpdatePasskeyQueryView};
use crate::endpoints::v1::auth::login::endpoint::generate_session;
use crate::endpoints::v1::auth::login::view::LoginResponseView;
use crate::endpoints::validation::ValidatedJson;
use crate::passkeys::{self, Passkeys};
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::two_factor;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::state::AppState;
use webauthn_rs::prelude::Passkey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasskeyLoginError {
    /// The instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`.
    Unavailable,
    /// The administrator does not accept passkeys.
    NotAllowed,
    /// Unknown, expired or replayed challenge, unknown passkey, wrong signature, counter that went
    /// back, other origin or `rpId`, passkey of an archived account: one answer for all of them.
    Refused,
    TooManyRequests(u64),
    Internal,
}

impl std::fmt::Display for PasskeyLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "Passkeys are not configured."),
            Self::NotAllowed => write!(f, "Passkeys are not allowed on this instance."),
            Self::Refused => write!(f, "Invalid passkey."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::Internal => write!(f, "An internal error occurred."),
        }
    }
}

impl ResponseError for PasskeyLoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotAllowed => StatusCode::FORBIDDEN,
            Self::Refused => StatusCode::UNAUTHORIZED,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Both routes spend the sign-in budget of the client address (shared with the password login).
fn spend_budget(
    limits: Option<&RateLimits>,
    ip: std::net::IpAddr,
) -> Result<(), PasskeyLoginError> {
    if let Some(limits) = limits {
        limits
            .login_per_ip
            .hit(&ip_key(ip))
            .map_err(|refused| PasskeyLoginError::TooManyRequests(refused.retry_after_seconds))?;
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "/options",
    summary = "Passkey sign-in: get the challenge",
    description = "Starts a discoverable passkey sign-in: the browser offers the passkeys it holds \
                   for this site, so nobody types an e-mail (and no account can be enumerated). \
                   Returns the WebAuthn options (user verification required) and the `id` of the \
                   ceremony, kept two minutes in Redis and usable once. Send the browser's answer \
                   to `POST /api/v1/auth/passkey`.\n\n\
                   Rate limited per client address, like the password login.",
    responses(
        (
            status = 200,
            description = "Challenge created.",
            body = PasskeyLoginOptionsView,
            example = json!({
                "id": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE",
                "options": {
                    "publicKey": {
                        "challenge": "q1w2e3r4t5y6u7i8o9p0a1s2d3f4g5h6j7k8l9z0x1c",
                        "timeout": 300_000,
                        "rpId": "mairie360-eip.fr",
                        "userVerification": "required"
                    },
                    "mediation": "conditional"
                }
            })
        ),
        (
            status = 403,
            description = "The administrator does not accept passkeys (two-factor policy).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not allowed on this instance.")
        ),
        (
            status = 429,
            description = "Too many attempts. The `Retry-After` header gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before the next attempt.")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Internal error: database or Redis.",
            body = String,
            content_type = "text/plain",
            example = json!("An internal error occurred.")
        ),
        (
            status = 503,
            description = "The instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`: the front falls back to the password.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/passkey/options")]
#[allow(clippy::future_not_send)]
pub async fn passkey_login_options(
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    relying_party: Option<web::Data<Passkeys>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeyLoginError> {
    spend_budget(limits.as_ref().map(web::Data::get_ref), client_ip(&request))?;
    let relying_party = relying_party.ok_or(PasskeyLoginError::Unavailable)?;
    let policy = two_factor::load_policy(state.get_smart_db())
        .await
        .map_err(|e| {
            tracing::error!("Passkey options policy DB Error: {e}");
            PasskeyLoginError::Internal
        })?;
    if !policy.passkey_allowed {
        return Err(PasskeyLoginError::NotAllowed);
    }
    let (options, ceremony) = relying_party
        .webauthn()
        .start_discoverable_authentication()
        .map_err(|e| {
            tracing::error!("Passkey options: {e}");
            PasskeyLoginError::Internal
        })?;
    let id = passkeys::begin_login(state.get_redis(), &ceremony)
        .await
        .map_err(|e| {
            tracing::error!("Passkey options Redis Error: {e}");
            PasskeyLoginError::Internal
        })?;
    let options = serde_json::to_value(options).map_err(|e| {
        tracing::error!("Passkey options serialization: {e}");
        PasskeyLoginError::Internal
    })?;
    Ok(HttpResponse::Ok().json(PasskeyLoginOptionsView { id, options }))
}

#[utoipa::path(
    post,
    path = "",
    summary = "Passkey sign-in: send the browser's answer",
    description = "Verifies the signed challenge (origin, `rpId`, user verification, signature, \
                   counter), finds the account by the passkey, then opens the session exactly like \
                   `POST /api/v1/auth/login`: JWT in the `Authorization` header, refresh token in \
                   the body. A passkey is a complete sign-in (it is both factors): it also \
                   satisfies a mandatory two-factor policy, and no code is asked.\n\n\
                   Every failure answers the same `401`: unknown, expired or replayed challenge \
                   (the challenge is burnt by the first attempt), unknown passkey, passkey of \
                   another account or of an archived one, bad signature, counter that went back, \
                   other origin or `rpId`.\n\n\
                   Rate limited per client address, like the password login.",
    request_body = PasskeyLoginView,
    responses(
        (
            status = 200,
            description = "Signed in. The JWT is in the `Authorization` header (`Bearer <jwt>`).",
            body = LoginResponseView,
            headers(("Authorization" = String, description = "`Bearer <jwt>`")),
            example = json!({ "refresh_token": "example-rotated-refresh-token" })
        ),
        (
            status = 400,
            description = "Malformed body or credential.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid body: missing field `credential`")
        ),
        (
            status = 401,
            description = "The passkey is refused (see above): one answer for every reason.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid passkey.")
        ),
        (
            status = 403,
            description = "The administrator does not accept passkeys (two-factor policy).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not allowed on this instance.")
        ),
        (
            status = 429,
            description = "Too many attempts. The `Retry-After` header gives the seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(("Retry-After" = u64, description = "Seconds to wait before the next attempt.")),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Internal error: database, Redis or JWT generation.",
            body = String,
            content_type = "text/plain",
            example = json!("An internal error occurred.")
        ),
        (
            status = 503,
            description = "The instance has no `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN`.",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/passkey")]
#[allow(clippy::future_not_send)]
pub async fn passkey_login(
    payload: ValidatedJson<PasskeyLoginView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    relying_party: Option<web::Data<Passkeys>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeyLoginError> {
    let view = payload.into_inner();
    let ip_address = client_ip(&request);
    spend_budget(limits.as_ref().map(web::Data::get_ref), ip_address)?;
    let relying_party = relying_party.ok_or(PasskeyLoginError::Unavailable)?;
    let db = state.get_smart_db();

    // The challenge is taken (and burnt) first: a failed attempt cannot be retried on it.
    let ceremony = passkeys::take_login(state.get_redis(), view.id())
        .await
        .ok_or(PasskeyLoginError::Refused)?;
    let (handle, credential_id) = relying_party
        .webauthn()
        .identify_discoverable_authentication(view.credential())
        .map_err(|_| PasskeyLoginError::Refused)?;
    let stored: PasskeyOwnerRecord = db
        .fetch_one(&FindPasskeyQueryView::new(
            &general_purpose::STANDARD.encode(credential_id),
        ))
        .await
        .map_err(|_| PasskeyLoginError::Refused)?;
    let user_id = u64::try_from(stored.user_id).map_err(|_| PasskeyLoginError::Refused)?;
    // The handle the authenticator stores must be the one of the account that owns the passkey.
    if handle != passkeys::user_handle(user_id) {
        tracing::warn!(user_id, "Passkey sign-in with a foreign user handle");
        return Err(PasskeyLoginError::Refused);
    }
    let mut passkey: Passkey =
        serde_json::from_value(stored.passkey).map_err(|_| PasskeyLoginError::Refused)?;

    let policy = two_factor::load_policy(db).await.map_err(|e| {
        tracing::error!("Passkey sign-in policy DB Error: {e}");
        PasskeyLoginError::Internal
    })?;
    if !policy.passkey_allowed {
        return Err(PasskeyLoginError::NotAllowed);
    }

    let result = relying_party
        .webauthn()
        .finish_discoverable_authentication_v2(view.credential(), ceremony, &passkey)
        .map_err(|e| {
            tracing::warn!(user_id, "Passkey sign-in refused: {e}");
            PasskeyLoginError::Refused
        })?;
    // The counter (and the backup state) move forward: a clone that replays an older one is refused
    // by the next sign-in.
    if passkey.update_credential(&result).is_some() {
        let saved = serde_json::to_string(&passkey).map_err(|e| {
            tracing::error!("Passkey serialization: {e}");
            PasskeyLoginError::Internal
        })?;
        db.fetch_one::<crate::database::passkeys::UpdatedPasskeyRecord, _>(
            &UpdatePasskeyQueryView::new(stored.id, &saved),
        )
        .await
        .map_err(|e| {
            tracing::error!("Passkey update DB Error: {e}");
            PasskeyLoginError::Internal
        })?;
    }

    let (jwt, refresh_token) = generate_session(user_id, view.device_info(), ip_address, state)
        .await
        .map_err(|e| {
            tracing::error!("Passkey sign-in session error: {e:?}");
            PasskeyLoginError::Internal
        })?;
    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token)))
}
