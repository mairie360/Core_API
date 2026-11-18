use super::view::{PasskeySetupCompletedView, PasskeySetupView};
use crate::client_ip::client_ip;
use crate::emails::{AccountEmails, PasskeyChange};
use crate::endpoints::v1::auth::login::endpoint::generate_session;
use crate::endpoints::v1::auth::two_factor_setup::endpoint::{open_setup, SetupLoginError};
use crate::endpoints::v1::auth::two_factor_setup::view::SetupTokenView;
use crate::endpoints::v1::user::me::passkeys::endpoint::{
    ensure_available, finish_registration, notify, start_registration, PasskeyError,
};
use crate::endpoints::v1::user::me::passkeys::view::PasskeyRegisterOptionsView;
use crate::endpoints::validation::ValidatedJson;
use crate::passkeys::Passkeys;
use crate::rate_limit::RateLimits;
use crate::two_factor;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

/// A refusal of the enrolment by passkey: the token side or the passkey side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasskeySetupError {
    Token(SetupLoginError),
    Passkey(PasskeyError),
}

impl std::fmt::Display for PasskeySetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token(e) => e.fmt(f),
            Self::Passkey(e) => e.fmt(f),
        }
    }
}

impl ResponseError for PasskeySetupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Token(e) => e.status_code(),
            Self::Passkey(e) => e.status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Token(e) => e.error_response(),
            Self::Passkey(e) => e.error_response(),
        }
    }
}

impl From<SetupLoginError> for PasskeySetupError {
    fn from(error: SetupLoginError) -> Self {
        Self::Token(error)
    }
}

impl From<PasskeyError> for PasskeySetupError {
    fn from(error: PasskeyError) -> Self {
        Self::Passkey(error)
    }
}

#[utoipa::path(
    post,
    path = "/setup/options",
    summary = "Mandatory enrolment: get the challenge to register a passkey",
    description = "Starts the registration of a passkey for an agent who must set up a second \
                   factor (`2fa_setup_required` with `passkey_allowed: true`). Public route: the \
                   enrolment token is the credential. Returns the `WebAuthn` creation options \
                   (resident key and user verification required, no attestation, so synchronised \
                   passkeys are accepted). The ceremony is kept two minutes and read once; send \
                   the browser's answer to `POST /api/v1/auth/passkey/setup`.\n\n\
                   Rate limited: 30 attempts per minute per client address and 10 per 15 minutes \
                   per account.",
    request_body = SetupTokenView,
    responses(
        (status = 200, description = "Challenge created.", body = PasskeyRegisterOptionsView),
        (
            status = 400,
            description = "Malformed JSON body, missing field, or a value longer than its limit or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `token`: must not contain control characters")
        ),
        (
            status = 401,
            description = "Unknown, expired, spent or burnt enrolment token.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid or expired enrolment token.")
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
            description = "Database or Redis error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
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
#[post("/passkey/setup/options")]
#[allow(clippy::future_not_send)]
pub async fn passkey_setup_options(
    payload: ValidatedJson<SetupTokenView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    relying_party: Option<web::Data<Passkeys>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeySetupError> {
    let view = payload.into_inner();
    let pending = open_setup(
        &state,
        limits.as_ref().map(web::Data::get_ref),
        client_ip(&request),
        view.token(),
    )
    .await?;
    let relying_party = ensure_available(&state, relying_party.as_ref()).await?;
    let options = start_registration(&state, relying_party, pending.user_id).await?;
    Ok(HttpResponse::Ok().json(options))
}

#[utoipa::path(
    post,
    path = "/setup",
    summary = "Mandatory enrolment: register a passkey and open the session",
    description = "Verifies the answer to the challenge of `POST /api/v1/auth/passkey/setup/options` \
                   (origin, `rpId`, user verification), stores the passkey under the given label, \
                   **opens the session** (JWT in the `Authorization` header, refresh token in the \
                   body, like a login). A passkey is a complete second factor: no backup codes are \
                   issued. Public route; the enrolment token is spent on success and burnt after \
                   five refused answers (the agent starts over with the password). The agent \
                   receives a security e-mail (device, date, address).\n\n\
                   Rate limited: 30 attempts per minute per client address and 10 per 15 minutes \
                   per account.",
    request_body = PasskeySetupView,
    responses(
        (
            status = 200,
            description = "Passkey registered and session opened. The JWT is returned in the `Authorization` header.",
            body = PasskeySetupCompletedView,
            headers(("Authorization" = String, description = "Access JWT, prefixed with `Bearer `."))
        ),
        (
            status = 400,
            description = "Malformed body, empty or too long label, or no pending registration (never started, expired, already used) or an answer that does not verify.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid or expired passkey registration.")
        ),
        (
            status = 401,
            description = "Unknown, expired, spent or burnt enrolment token.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid or expired enrolment token.")
        ),
        (
            status = 403,
            description = "The administrator does not accept passkeys (two-factor policy).",
            body = String,
            content_type = "text/plain",
            example = json!("Passkeys are not allowed on this instance.")
        ),
        (
            status = 409,
            description = "This credential is already registered.",
            body = String,
            content_type = "text/plain",
            example = json!("This passkey is already registered.")
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
            description = "Database, Redis or JWT generation error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
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
#[post("/passkey/setup")]
#[allow(clippy::future_not_send)]
pub async fn passkey_setup(
    payload: ValidatedJson<PasskeySetupView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    relying_party: Option<web::Data<Passkeys>>,
    emails: Option<web::Data<AccountEmails>>,
    request: HttpRequest,
) -> Result<impl Responder, PasskeySetupError> {
    let view = payload.into_inner();
    let ip_address = client_ip(&request);
    let pending = open_setup(
        &state,
        limits.as_ref().map(web::Data::get_ref),
        ip_address,
        view.token(),
    )
    .await?;
    let relying_party = ensure_available(&state, relying_party.as_ref()).await?;
    let user_id = pending.user_id;
    let device_info = pending.device_info.clone();
    let redis = state.get_redis();
    let stored = match finish_registration(
        &state,
        relying_party,
        user_id,
        view.label(),
        view.credential(),
    )
    .await
    {
        Ok(stored) => stored,
        Err(PasskeyError::InvalidRegistration) => {
            // A refused answer counts like a wrong code: the token is burnt at the fifth.
            let burnt = two_factor::record_setup_failure(redis, view.token(), pending).await;
            return Err(if burnt {
                SetupLoginError::InvalidToken.into()
            } else {
                PasskeyError::InvalidRegistration.into()
            });
        }
        Err(e) => return Err(e.into()),
    };
    // The passkey is stored: the agent is told whatever happens next (token race, session failure).
    notify(
        &state,
        emails.as_ref().map(web::Data::get_ref),
        user_id,
        PasskeyChange::Added,
        &stored.label,
        &request,
    )
    .await;
    match two_factor::consume_setup(redis, view.token()).await {
        Ok(true) => {}
        // Another request spent the token first.
        Ok(false) => return Err(SetupLoginError::InvalidToken.into()),
        Err(e) => {
            tracing::error!("Passkey enrolment: could not spend the token: {e}");
            return Err(SetupLoginError::Internal.into());
        }
    }
    let (jwt, refresh_token) = generate_session(user_id, &device_info, ip_address, state.clone())
        .await
        .map_err(|e| {
            tracing::error!("Passkey enrolment session error: {e:?}");
            SetupLoginError::Internal
        })?;
    tracing::info!(user_id, "Mandatory second factor enrolled with a passkey");
    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(PasskeySetupCompletedView { refresh_token }))
}
