use super::view::{SetupCompletedView, SetupConfirmView, SetupTokenView};
use crate::client_ip::client_ip;
use crate::endpoints::v1::auth::login::endpoint::generate_session;
use crate::endpoints::v1::user::me::two_factor::endpoint::{
    confirm_enrolment, start_enrolment, TwoFactorError,
};
use crate::endpoints::v1::user::me::two_factor::view::TwoFactorSetupView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, user_key, RateLimits};
use crate::second_factor::SecretCipher;
use crate::two_factor::{self, PendingLogin};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupLoginError {
    InvalidToken,
    InvalidCode,
    NotStarted,
    AlreadyEnabled,
    TooManyRequests(u64),
    Unavailable,
    Internal,
}

impl std::fmt::Display for SetupLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "Invalid or expired enrolment token."),
            Self::InvalidCode => write!(f, "Invalid verification code."),
            Self::NotStarted => write!(f, "No enrolment in progress."),
            Self::AlreadyEnabled => write!(f, "Two-factor authentication is already enabled."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::Unavailable => write!(f, "Two-factor authentication is not configured."),
            Self::Internal => write!(f, "An internal error occurred."),
        }
    }
}

impl ResponseError for SetupLoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidToken | Self::InvalidCode => StatusCode::UNAUTHORIZED,
            Self::NotStarted => StatusCode::NOT_FOUND,
            Self::AlreadyEnabled => StatusCode::CONFLICT,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
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

impl From<TwoFactorError> for SetupLoginError {
    fn from(error: TwoFactorError) -> Self {
        match error {
            TwoFactorError::AlreadyEnabled => Self::AlreadyEnabled,
            TwoFactorError::NotEnabled => Self::NotStarted,
            TwoFactorError::InvalidCredentials => Self::InvalidCode,
            TwoFactorError::Unavailable => Self::Unavailable,
            TwoFactorError::TooManyRequests(seconds) => Self::TooManyRequests(seconds),
            TwoFactorError::Mandatory | TwoFactorError::DatabaseError => Self::Internal,
        }
    }
}

/// Counts the attempt against the per-address and per-account budgets and reads the token.
pub(crate) async fn open_setup(
    state: &AppState,
    limits: Option<&RateLimits>,
    ip_address: std::net::IpAddr,
    token: &str,
) -> Result<PendingLogin, SetupLoginError> {
    if let Some(limits) = limits {
        limits
            .login_2fa_per_ip
            .hit(&ip_key(ip_address))
            .map_err(|refused| SetupLoginError::TooManyRequests(refused.retry_after_seconds))?;
    }
    let pending = two_factor::read_setup(state.get_redis(), token)
        .await
        .ok_or(SetupLoginError::InvalidToken)?;
    if let Some(limits) = limits {
        limits
            .login_2fa_per_user
            .hit(&user_key(pending.user_id))
            .map_err(|refused| SetupLoginError::TooManyRequests(refused.retry_after_seconds))?;
    }
    Ok(pending)
}

#[utoipa::path(
    post,
    path = "/setup",
    summary = "Mandatory enrolment: get the secret of the authenticator application",
    description = "For an agent who got `202` `2fa_setup_required` (the instance makes the second \
                   factor mandatory and the account has none). Starts the enrolment: returns the \
                   secret and the `otpauth://` URI to add to an authenticator application. Calling \
                   it again replaces the pending secret. Public route: the enrolment token is the \
                   credential, it lives five minutes and opens no other route.\n\n\
                   Rate limited: 30 attempts per minute per client address and 10 per 15 minutes \
                   per account.",
    request_body(
        content = SetupTokenView,
        description = "Enrolment token.",
        example = json!({ "token": "example-two-factor-token" })
    ),
    responses(
        (
            status = 200,
            description = "Secret of the pending enrolment.",
            body = TwoFactorSetupView,
            example = json!({
                "otpauth_uri": "otpauth://totp/Mairie360:agent%40mairie.fr?secret=JBSWY3DPEHPK3PXP&issuer=Mairie360",
                "secret": "JBSWY3DPEHPK3PXP"
            })
        ),
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
            status = 409,
            description = "The account already has an active second factor.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is already enabled.")
        ),
        (
            status = 429,
            description = "Too many attempts from this client address or for this account. The `Retry-After` header gives the number of seconds to wait.",
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
            description = "The instance has no `TOTP_ENCRYPTION_KEY`.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/2fa/setup")]
#[allow(clippy::future_not_send)]
pub async fn setup_login_two_factor(
    payload: ValidatedJson<SetupTokenView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    secrets: Option<web::Data<SecretCipher>>,
    request: HttpRequest,
) -> Result<impl Responder, SetupLoginError> {
    let view = payload.into_inner();
    let secrets = secrets.ok_or(SetupLoginError::Unavailable)?;
    let pending = open_setup(
        &state,
        limits.as_ref().map(web::Data::get_ref),
        client_ip(&request),
        view.token(),
    )
    .await?;
    let setup = start_enrolment(&state, &secrets, pending.user_id).await?;
    Ok(HttpResponse::Ok().json(setup))
}

#[utoipa::path(
    post,
    path = "/confirm",
    summary = "Mandatory enrolment: confirm with a first code and open the session",
    description = "Finishes the enrolment started with `POST /api/v1/auth/2fa/setup`: checks the \
                   first code of the authenticator application, activates the second factor, \
                   **opens the session** (JWT in the `Authorization` header, refresh token in the \
                   body, like a login) and returns the ten single-use backup codes: **the only \
                   time they are visible**. Public route; the enrolment token is spent on \
                   success and burnt after five wrong codes (the agent starts over with the \
                   password).\n\n\
                   Rate limited: 30 attempts per minute per client address and 10 per 15 minutes \
                   per account.",
    request_body(
        content = SetupConfirmView,
        description = "Enrolment token and first code.",
        example = json!({ "token": "example-two-factor-token", "code": "492039" })
    ),
    responses(
        (
            status = 200,
            description = "Second factor active and session opened. The JWT is returned in the `Authorization` header.",
            body = SetupCompletedView,
            headers(("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")),
            example = json!({
                "refresh_token": "example-rotated-refresh-token",
                "backup_codes": ["a1b2-c3d4", "e5f6-0718", "9a8b-7c6d", "1e2f-3a4b", "5c6d-7e8f", "0a1b-2c3d", "4e5f-6a7b", "8c9d-0e1f", "2a3b-4c5d", "6e7f-8a9b"]
            })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, or a value longer than its limit or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `code`: must not contain control characters")
        ),
        (
            status = 401,
            description = "`Invalid or expired enrolment token.`: unknown, expired, spent or burnt token. `Invalid verification code.`: wrong code; the token stays valid until the fifth failure.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid verification code.")
        ),
        (
            status = 404,
            description = "No enrolment in progress: call `POST /api/v1/auth/2fa/setup` first.",
            body = String,
            content_type = "text/plain",
            example = json!("No enrolment in progress.")
        ),
        (
            status = 429,
            description = "Too many attempts from this client address or for this account. The `Retry-After` header gives the number of seconds to wait.",
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
            description = "The instance has no `TOTP_ENCRYPTION_KEY`.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/2fa/confirm")]
#[allow(clippy::future_not_send)]
pub async fn confirm_login_two_factor(
    payload: ValidatedJson<SetupConfirmView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    secrets: Option<web::Data<SecretCipher>>,
    request: HttpRequest,
) -> Result<impl Responder, SetupLoginError> {
    let view = payload.into_inner();
    let ip_address = client_ip(&request);
    let secrets = secrets.ok_or(SetupLoginError::Unavailable)?;
    let pending = open_setup(
        &state,
        limits.as_ref().map(web::Data::get_ref),
        ip_address,
        view.token(),
    )
    .await?;
    let redis = state.get_redis();
    let backup_codes = match confirm_enrolment(&state, &secrets, pending.user_id, view.code()).await
    {
        Ok(codes) => codes,
        Err(TwoFactorError::InvalidCredentials) => {
            tracing::warn!(
                user_id = pending.user_id,
                "Mandatory enrolment code refused"
            );
            let burnt = two_factor::record_setup_failure(redis, view.token(), pending).await;
            return Err(if burnt {
                SetupLoginError::InvalidToken
            } else {
                SetupLoginError::InvalidCode
            });
        }
        Err(e) => return Err(e.into()),
    };
    match two_factor::consume_setup(redis, view.token()).await {
        Ok(true) => {}
        // Another request with a valid code spent the token first.
        Ok(false) => return Err(SetupLoginError::InvalidToken),
        Err(e) => {
            tracing::error!("Enrolment: could not spend the token: {e}");
            return Err(SetupLoginError::Internal);
        }
    }
    let user_id = pending.user_id;
    let (jwt, refresh_token) =
        generate_session(user_id, &pending.device_info, ip_address, state.clone())
            .await
            .map_err(|e| {
                tracing::error!("Enrolment session error: {e:?}");
                SetupLoginError::Internal
            })?;
    tracing::info!(user_id, "Mandatory second factor enrolled");
    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(SetupCompletedView {
            refresh_token,
            backup_codes,
        }))
}
