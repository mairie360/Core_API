use crate::client_ip::client_ip;
use crate::database::auth::active_user_id::GetActiveUserIdByEmailQueryView;
use crate::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use crate::endpoints::v1::auth::login::endpoint::{
    generate_session, two_factor_challenge, LoginError,
};
use crate::endpoints::v1::auth::login::view::TwoFactorRequiredView;
use crate::endpoints::v1::auth::reset_password::view::{
    ResetPasswordResponseView, ResetPasswordView,
};
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::second_factor::SecretCipher;
use crate::session_revocation::{publish_revoked_sessions, revoke_all_user_sessions_in};
use crate::two_factor;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
enum ResetPasswordError {
    DatabaseError,
    RedisError,
    TokenGenerationError,
    TooManyRequests(u64),
    UnknownToken,
    /// The password is reset but the account has a second factor: the login token of the second
    /// step, no session (MAIR-805).
    TwoFactorRequired(String),
    TwoFactorUnavailable,
    /// The password is reset (and the other sessions revoked) but the second step could not be
    /// opened: the answer says so, the agent logs in with the new password.
    SecondStepFailed,
}

impl std::fmt::Display for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError | Self::RedisError | Self::TokenGenerationError => {
                write!(
                    f,
                    "The password could not be reset, please try again later."
                )
            }
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::UnknownToken => {
                write!(f, "Unknown token")
            }
            Self::SecondStepFailed => write!(
                f,
                "The password was reset, but the second step could not be opened: \
                 log in with the new password."
            ),
            Self::TwoFactorRequired(_) => write!(f, "Second factor required."),
            Self::TwoFactorUnavailable => {
                write!(f, "Two-factor authentication is not configured.")
            }
        }
    }
}

impl ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError
            | Self::RedisError
            | Self::TokenGenerationError
            | Self::SecondStepFailed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::UnknownToken => StatusCode::UNAUTHORIZED,
            Self::TwoFactorRequired(_) => StatusCode::ACCEPTED,
            Self::TwoFactorUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        if let Self::TwoFactorRequired(token) = self {
            return HttpResponse::build(self.status_code())
                .json(TwoFactorRequiredView::new(token.clone()));
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Id of the account the token was issued for. An account archived since the e-mail was sent
/// answers like an unknown token.
async fn get_user_id(smart_db: &SmartDatabase, email: &str) -> Result<u64, ResetPasswordError> {
    let user_id = smart_db
        .fetch_scalar::<i32, _>(&GetActiveUserIdByEmailQueryView::new(email))
        .await
        .map_err(|e| {
            tracing::error!("Reset password DB Error: {e}");
            ResetPasswordError::DatabaseError
        })?;
    u64::try_from(user_id)
        .ok()
        .filter(|id| *id > 0)
        .ok_or(ResetPasswordError::UnknownToken)
}

/// Sets the new password and revokes every session opened with the previous one, in one
/// transaction (MAIR-420). Returns the revoked sessions, to publish once committed.
async fn reset_pwd(
    smart_db: &SmartDatabase,
    new_password: &str,
    user_id: u64,
) -> Result<Vec<Uuid>, ResetPasswordError> {
    let hashed_password = passwords::hash(new_password).await.map_err(|e| {
        tracing::error!("Password hashing error: {e}");
        ResetPasswordError::DatabaseError
    })?;
    let database_error = |e: ApiLibError| {
        tracing::error!("Reset password DB Error: {e}");
        ResetPasswordError::DatabaseError
    };
    let mut tx = smart_db.begin().await.map_err(database_error)?;
    // Proving ownership of the mailbox also completes a pending first connection: the account no
    // longer asks for a password change at the next login (MAIR-390).
    let _: i64 = tx
        .fetch_scalar(&UnsetFirstConnectionQueryView::new(
            user_id,
            &hashed_password,
        ))
        .await
        .map_err(database_error)?;
    // Sessions opened with the old password end here, in every API (MAIR-264). The session opened
    // afterwards is the only one left.
    let revoked = revoke_all_user_sessions_in(&mut tx, user_id)
        .await
        .map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(revoked)
}

async fn reset_password_trigger(
    state: web::Data<AppState>,
    view: ResetPasswordView,
    ip_adress: std::net::IpAddr,
    can_verify_second_factor: bool,
) -> Result<(String, String), ResetPasswordError> {
    let smart_db = state.get_smart_db();
    let redis = state.get_redis();

    let key = format!("{}/forgot_password_email", view.token());
    let email: String = match redis.secure_get::<String>(&key).await {
        Ok(Some(email)) => email,
        other => {
            tracing::error!("Failed to get email from Redis: {other:?}");
            return Err(ResetPasswordError::UnknownToken);
        }
    };
    let user_id = get_user_id(smart_db, &email).await?;

    // Checked before anything is written: an instance that cannot verify the second factor of this
    // account answers `503` with the password, the sessions and the reset token untouched, so the
    // agent can retry once the instance is configured (MAIR-805).
    if !can_verify_second_factor {
        let protected = two_factor::is_enabled(smart_db, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Second factor status DB Error: {e}");
                ResetPasswordError::DatabaseError
            })?;
        if protected {
            tracing::error!(
                "User {user_id} has a second factor but TOTP_ENCRYPTION_KEY is not set"
            );
            return Err(ResetPasswordError::TwoFactorUnavailable);
        }
    }

    let reversed_key = format!("{email}/forgot_password_token");
    if let Err(e) = redis.delete(&reversed_key).await {
        tracing::error!("Failed to delete reversed key: {e:?}");
        return Err(ResetPasswordError::RedisError);
    }
    if let Err(e) = redis.delete(&key).await {
        tracing::error!("Failed to delete key: {e:?}");
        return Err(ResetPasswordError::RedisError);
    }

    let revoked = reset_pwd(smart_db, view.new_password(), user_id).await?;
    publish_revoked_sessions(redis, &revoked).await;

    // Owning the mailbox is not a second factor: a protected account goes through the second step
    // like a login (MAIR-805).
    match two_factor_challenge(
        &state,
        user_id,
        &view.device_info(),
        can_verify_second_factor,
    )
    .await
    {
        Ok(Some(token)) => return Err(ResetPasswordError::TwoFactorRequired(token)),
        Ok(None) => {}
        Err(LoginError::TwoFactorUnavailable) => {
            return Err(ResetPasswordError::TwoFactorUnavailable)
        }
        // The password is already changed: say so instead of a generic failure.
        Err(e) => {
            tracing::error!("Second factor check after a password reset: {e:?}");
            return Err(ResetPasswordError::SecondStepFailed);
        }
    }

    match generate_session(user_id, &view.device_info(), ip_adress, state).await {
        Ok((jwt, refresh_token)) => Ok((jwt, refresh_token)),
        Err(e) => {
            tracing::error!("Failed to generate session: {e:?}");
            Err(ResetPasswordError::TokenGenerationError)
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Reset a password with a token",
    description = "Consumes the token sent by `POST /api/v1/auth/forgot_password`, stores the new \
                   password and immediately opens a session: the response carries a JWT in the \
                   `Authorization` header and a refresh token in the body, like a login. A second \
                   call with the same token answers `401`.\n\n\
                   Every other session of the account is revoked, and an account still waiting \
                   for its first connection is activated: the next login no longer answers \
                   `412`.\n\n\
                   Public route: `JwtMiddleware` lets everything under `/auth` through. Rate \
                   limited together with `force_change_password`: 10 requests per 15 minutes per \
                   client address, `429` beyond.",
    request_body(
        content = ResetPasswordView,
        description = "Token received by e-mail, new password and description of the device.",
        example = json!({
            "token": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
            "new_password": "NouveauMotDePasse!123",
            "device_info": "Chrome 140 on Windows 11"
        })
    ),
    responses(
        (
            status = 200,
            description = "Password reset and session opened.",
            body = ResetPasswordResponseView,
            headers(
                ("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")
            ),
            example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
        ),
        (
            status = 202,
            description = "Password reset, but the account is protected by the second factor: no session is opened (the mailbox is not a second factor). Call `POST /api/v1/auth/login/2fa` with the returned token within five minutes.",
            body = TwoFactorRequiredView,
            example = json!({ "status": "2fa_required", "token": "example-two-factor-token" })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, `token` or `device_info` longer than 512 characters or containing a control character, or `new_password` shorter than 8 characters, longer than 255 characters or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `new_password`: must be at least 8 characters")
        ),
        (
            status = 401,
            description = "Unknown, expired or already consumed token, or account archived since the token was sent.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown token")
        ),
        (
            status = 429,
            description = "Too many requests from this client address. The `Retry-After` header gives the number of seconds to wait.",
            body = String,
            content_type = "text/plain",
            headers(
                ("Retry-After" = u64, description = "Seconds to wait before the next request.")
            ),
            example = json!("Too many requests, please try again later.")
        ),
        (
            status = 500,
            description = "Database or Redis failure, or JWT generation failure. When the password was already reset and only the second step could not be opened, the text says so: log in with the new password (the reset token is spent).",
            body = String,
            content_type = "text/plain",
            example = json!("The password could not be reset, please try again later.")
        ),
        (
            status = 503,
            description = "The account has a second factor but the instance has no `TOTP_ENCRYPTION_KEY`. Nothing is changed: the password, the sessions and the reset token are untouched, so the reset can be retried once the instance is configured.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Auth",
)]
#[post("/reset_password")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn reset_password(
    state: web::Data<AppState>,
    body: ValidatedJson<ResetPasswordView>,
    limits: Option<web::Data<RateLimits>>,
    secrets: Option<web::Data<SecretCipher>>,
    request: HttpRequest,
) -> Result<impl Responder, ResetPasswordError> {
    let ip_address = client_ip(&request);
    if let Some(limits) = limits {
        limits
            .password_token_per_ip
            .hit(&ip_key(ip_address))
            .map_err(|refused| ResetPasswordError::TooManyRequests(refused.retry_after_seconds))?;
    }
    let (jwt, refresh_token) =
        reset_password_trigger(state, body.into_inner(), ip_address, secrets.is_some()).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(ResetPasswordResponseView::from(refresh_token)))
}
