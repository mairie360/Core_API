use super::view::LoginTwoFactorView;
use crate::client_ip::client_ip;
use crate::database::second_factor::{LockActiveTotpQueryView, Written};
use crate::endpoints::v1::auth::login::endpoint::generate_session_in;
use crate::endpoints::v1::auth::login::view::LoginResponseView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, user_key, RateLimits};
use crate::second_factor::SecretCipher;
use crate::two_factor::{self, VerifyError};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginTwoFactorError {
    /// Unknown, expired, spent or burnt login token.
    InvalidToken,
    /// Wrong code; the login token stays valid.
    InvalidCode,
    /// Backup code already spent; the login token stays valid.
    SpentBackupCode,
    TooManyRequests(u64),
    Unavailable,
    Internal,
}

impl std::fmt::Display for LoginTwoFactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "Invalid or expired login token."),
            Self::InvalidCode => write!(f, "Invalid verification code."),
            Self::SpentBackupCode => write!(f, "Backup code already used."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::Unavailable => write!(f, "Two-factor authentication is not configured."),
            Self::Internal => write!(f, "An internal error occurred."),
        }
    }
}

impl ResponseError for LoginTwoFactorError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidToken | Self::InvalidCode | Self::SpentBackupCode => {
                StatusCode::UNAUTHORIZED
            }
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

#[utoipa::path(
    post,
    path = "",
    summary = "Log in, second step",
    description = "Finishes a login that answered `202` (`POST /api/v1/auth/login`, or \
                   `POST /api/v1/auth/reset_password` on a protected account): checks the code \
                   against the login token, then opens the session like a login (JWT in the \
                   `Authorization` header, refresh token in the body). Public route.\n\n\
                   The code is a 6-digit code of the authenticator application, or a backup code. \
                   A code is accepted only once: replaying the code of the previous step, or a \
                   spent backup code (answered apart as `Backup code already used.`), is refused.\n\n\
                   The login token lives five minutes, opens no other route, and is burnt after \
                   five wrong codes or one success: the agent then starts over with the password.\n\n\
                   Rate limited: 30 attempts per minute per client address and 10 per 15 minutes \
                   per account, successful or not; beyond that the answer is `429`.",
    request_body(
        content = LoginTwoFactorView,
        description = "Login token and code.",
        example = json!({ "token": "example-two-factor-token", "code": "492039" })
    ),
    responses(
        (
            status = 200,
            description = "Logged in. The JWT is returned in the `Authorization` header, the refresh token in the body.",
            body = LoginResponseView,
            headers(("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")),
            example = json!({ "refresh_token": "example-rotated-refresh-token" })
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
            description = "`Invalid or expired login token.`: unknown, expired, spent or burnt token, or the second factor was reset meanwhile. `Invalid verification code.`: wrong or replayed code. `Backup code already used.`: the code is a backup code of the account that was already spent, the agent picks another one. For both the token stays valid until the fifth failure, which counts like any wrong code.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid verification code.")
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
#[post("/login/2fa")]
#[allow(clippy::future_not_send)]
pub async fn login_two_factor(
    payload: ValidatedJson<LoginTwoFactorView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    secrets: Option<web::Data<SecretCipher>>,
    relying_party: Option<web::Data<crate::passkeys::Passkeys>>,
    request: HttpRequest,
) -> Result<impl Responder, LoginTwoFactorError> {
    let view = payload.into_inner();
    let ip_address = client_ip(&request);
    let limits = limits.as_ref().map(web::Data::get_ref);
    if let Some(limits) = limits {
        limits
            .login_2fa_per_ip
            .hit(&ip_key(ip_address))
            .map_err(|refused| LoginTwoFactorError::TooManyRequests(refused.retry_after_seconds))?;
    }
    let secrets = secrets.ok_or(LoginTwoFactorError::Unavailable)?;
    let redis = state.get_redis();
    let pending = two_factor::read_login(redis, view.token())
        .await
        .ok_or(LoginTwoFactorError::InvalidToken)?;
    if let Some(limits) = limits {
        limits
            .login_2fa_per_user
            .hit(&user_key(pending.user_id))
            .map_err(|refused| LoginTwoFactorError::TooManyRequests(refused.retry_after_seconds))?;
    }

    // The modes the administrator accepts now, not the ones of the day the password was typed.
    let policy = two_factor::load_policy(state.get_smart_db())
        .await
        .map_err(|e| {
            tracing::error!("Second step policy DB Error: {e}");
            LoginTwoFactorError::Internal
        })?;
    match two_factor::verify_code_under_policy(
        state.get_smart_db(),
        &secrets,
        &policy,
        pending.user_id,
        view.code(),
    )
    .await
    {
        Ok(()) => {}
        Err(error @ (VerifyError::InvalidCode | VerifyError::SpentBackupCode)) => {
            tracing::warn!(user_id = pending.user_id, "Login second factor refused");
            let burnt = two_factor::record_login_failure(redis, view.token(), pending).await;
            return Err(if burnt {
                LoginTwoFactorError::InvalidToken
            } else if error == VerifyError::SpentBackupCode {
                LoginTwoFactorError::SpentBackupCode
            } else {
                LoginTwoFactorError::InvalidCode
            });
        }
        // Reset by an administrator since the password step: nothing left to verify.
        Err(VerifyError::NotEnabled) => return Err(LoginTwoFactorError::InvalidToken),
        Err(VerifyError::Internal) => return Err(LoginTwoFactorError::Internal),
    }
    match two_factor::consume_login(redis, view.token()).await {
        Ok(true) => {}
        // Another request with a valid code spent the token first: it gets the session.
        Ok(false) => return Err(LoginTwoFactorError::InvalidToken),
        Err(e) => {
            tracing::error!("Second step: could not spend the login token: {e}");
            return Err(LoginTwoFactorError::Internal);
        }
    }

    // The session is written under a shared lock on the factor, in one transaction: an
    // administrator reset (which deletes the factor, then revokes the sessions) either waits for
    // this commit and revokes the session too, or has committed first and the lock finds nothing.
    // A session can no longer outlive the factor that authorised it.
    let mut tx = state.get_smart_db().begin().await.map_err(|e| {
        tracing::error!("Second step: begin: {e}");
        LoginTwoFactorError::Internal
    })?;
    match tx
        .fetch_one::<Written, _>(&LockActiveTotpQueryView::new(pending.user_id))
        .await
    {
        Ok(_) => {}
        Err(ApiLibError::Database(DbError::NotFound)) => {
            return Err(LoginTwoFactorError::InvalidToken)
        }
        Err(e) => {
            tracing::error!("Second step: lock the factor: {e}");
            return Err(LoginTwoFactorError::Internal);
        }
    }
    let (jwt, refresh_token) = generate_session_in(
        &mut tx,
        pending.user_id,
        &pending.device_info,
        ip_address,
        &state,
    )
    .await
    .map_err(|e| {
        tracing::error!("Second step session error: {e:?}");
        LoginTwoFactorError::Internal
    })?;
    tx.commit().await.map_err(|e| {
        tracing::error!("Second step: commit: {e}");
        LoginTwoFactorError::Internal
    })?;
    let offer = crate::endpoints::v1::auth::login::endpoint::passkey_offer(
        &state,
        relying_party.is_some(),
        pending.user_id,
    )
    .await;
    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token).with_passkey(offer)))
}
