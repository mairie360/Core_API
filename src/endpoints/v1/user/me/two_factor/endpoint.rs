use super::view::{
    BackupCodesView, DisableTwoFactorView, TwoFactorCodeView, TwoFactorSetupView,
    TwoFactorStatusView,
};
use crate::database::auth::user_password::{GetUserPasswordQueryView, UserPassword};
use crate::database::ids::id_to_sql;
use crate::database::second_factor::{
    AccountEmail, ConfirmTotpQueryView, DeleteBackupCodesQueryView, DeleteTotpQueryView,
    GetAccountEmailQueryView, GetTotpQueryView, StartTotpEnrolmentQueryView, TotpRecord, Written,
};
use crate::endpoints::v1::auth::login::endpoint::is_password_valid;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{too_many_requests_response, user_key, RateLimits};
use crate::second_factor::{self, SecretCipher};
use crate::two_factor::{self, VerifyError};
use actix_web::http::StatusCode;
use actix_web::{get, post, web, HttpResponse, Responder, ResponseError};
use base64::{engine::general_purpose, Engine as _};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwoFactorError {
    /// `TOTP_ENCRYPTION_KEY` is not configured.
    Unavailable,
    /// The second factor is already active.
    AlreadyEnabled,
    /// No second factor to act on, or no enrolment started.
    NotEnabled,
    /// Wrong code or wrong password.
    InvalidCredentials,
    TooManyRequests(u64),
    DatabaseError,
}

impl std::fmt::Display for TwoFactorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "Two-factor authentication is not configured."),
            Self::AlreadyEnabled => write!(f, "Two-factor authentication is already enabled."),
            Self::NotEnabled => write!(f, "Two-factor authentication is not enabled."),
            Self::InvalidCredentials => write!(f, "Invalid code or password."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
        }
    }
}

impl ResponseError for TwoFactorError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::AlreadyEnabled => StatusCode::CONFLICT,
            Self::NotEnabled => StatusCode::NOT_FOUND,
            Self::InvalidCredentials => StatusCode::BAD_REQUEST,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

impl From<VerifyError> for TwoFactorError {
    fn from(error: VerifyError) -> Self {
        match error {
            VerifyError::InvalidCode => Self::InvalidCredentials,
            VerifyError::NotEnabled => Self::NotEnabled,
            VerifyError::Internal => Self::DatabaseError,
        }
    }
}

fn db_error(context: &str, e: &ApiLibError) -> TwoFactorError {
    tracing::error!("Two-factor {context}: {e}");
    TwoFactorError::DatabaseError
}

/// Every code typed on these routes spends the budget of the user, right or wrong.
fn spend_budget(limits: Option<&RateLimits>, user_id: u64) -> Result<(), TwoFactorError> {
    if let Some(limits) = limits {
        limits
            .second_factor_per_user
            .hit(&user_key(user_id))
            .map_err(|refused| TwoFactorError::TooManyRequests(refused.retry_after_seconds))?;
    }
    Ok(())
}

fn cipher(cipher: Option<&web::Data<SecretCipher>>) -> Result<&SecretCipher, TwoFactorError> {
    cipher
        .map(web::Data::get_ref)
        .ok_or(TwoFactorError::Unavailable)
}

#[utoipa::path(
    get,
    path = "/2fa/",
    summary = "Read the state of one's two-factor authentication",
    description = "Tells whether the caller's second factor (authenticator application) is \
                   active and how many backup codes are left. The secret and the codes are never \
                   returned here.",
    responses(
        (
            status = 200,
            description = "State of the second factor.",
            body = TwoFactorStatusView,
            example = json!({ "enabled": true, "backup_codes_remaining": 8 })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 500,
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[get("/2fa/")]
pub async fn get_two_factor_status(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<impl Responder, TwoFactorError> {
    let db = state.get_smart_db();
    let enabled = two_factor::is_enabled(db, auth_user.id)
        .await
        .map_err(|e| db_error("status", &e))?;
    let backup_codes_remaining = if enabled {
        two_factor::backup_codes_remaining(db, auth_user.id)
            .await
            .map_err(|e| db_error("backup code count", &e))?
    } else {
        0
    };
    Ok(HttpResponse::Ok().json(TwoFactorStatusView {
        enabled,
        backup_codes_remaining,
    }))
}

#[utoipa::path(
    post,
    path = "/2fa/setup/",
    summary = "Start the enrolment of the authenticator application",
    description = "Generates a new TOTP secret, keeps it pending (sealed, never stored in clear) \
                   and returns the `otpauth://` URI to show as a QR code, with the same secret in \
                   base32 for manual entry. The second factor is **not** active until \
                   `POST /api/v1/user/me/2fa/confirm/` receives a first valid code. Calling it \
                   again before the confirmation replaces the pending secret.\n\n\
                   Rate limited per user: twenty codes or enrolments per 15 minutes.",
    responses(
        (
            status = 200,
            description = "Pending secret created.",
            body = TwoFactorSetupView,
            example = json!({
                "otpauth_uri": "otpauth://totp/Mairie%20360%3Ajean%2Edupont%40mairie360%2Efr?secret=JBSWY3DPEHPK3PXP&issuer=Mairie%20360&algorithm=SHA1&digits=6&period=30",
                "secret": "JBSWY3DPEHPK3PXP"
            })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 409,
            description = "The second factor is already active: disable it first.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is already enabled.")
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
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The instance has no `TOTP_ENCRYPTION_KEY`: the second factor is not configured.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[post("/2fa/setup/")]
pub async fn setup_two_factor(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    secrets: Option<web::Data<SecretCipher>>,
    limits: Option<web::Data<RateLimits>>,
) -> Result<impl Responder, TwoFactorError> {
    let secrets = cipher(secrets.as_ref())?;
    spend_budget(limits.as_ref().map(web::Data::get_ref), auth_user.id)?;
    let db = state.get_smart_db();
    let account: AccountEmail = db
        .fetch_one(&GetAccountEmailQueryView::new(auth_user.id))
        .await
        .map_err(|e| db_error("read the account", &e))?;
    let secret = second_factor::generate_secret();
    let sealed = general_purpose::STANDARD.encode(secrets.seal(&secret));
    match db
        .fetch_one::<Written, _>(&StartTotpEnrolmentQueryView::new(auth_user.id, &sealed))
        .await
    {
        Ok(_) => {}
        Err(ApiLibError::Database(DbError::NotFound)) => {
            return Err(TwoFactorError::AlreadyEnabled)
        }
        Err(e) => return Err(db_error("start the enrolment", &e)),
    }
    Ok(HttpResponse::Ok().json(TwoFactorSetupView {
        otpauth_uri: second_factor::otpauth_uri(&secret, &account.email),
        secret: second_factor::secret_base32(&secret),
    }))
}

#[utoipa::path(
    post,
    path = "/2fa/confirm/",
    summary = "Confirm the enrolment with a first code",
    description = "Checks the first code of the authenticator application. When it is right, the \
                   second factor becomes active (from the next login) and ten single-use backup \
                   codes are returned: **this is the only time they are visible**.\n\n\
                   Rate limited per user: twenty codes or enrolments per 15 minutes.",
    request_body(
        content = TwoFactorCodeView,
        description = "The current code shown by the application.",
        example = json!({ "code": "492039" })
    ),
    responses(
        (
            status = 200,
            description = "Second factor active. The backup codes are shown once.",
            body = BackupCodesView,
            example = json!({ "backup_codes": ["k3m9-x7pq", "h4wn-2tre", "p8dc-m5zx", "w2ja-9ukb", "n7sy-4qfh", "t6ge-3xvr", "c5mz-8hwd", "y9pk-6nja", "r3fb-7csm", "x4qh-2yte"] })
        ),
        (
            status = 400,
            description = "Wrong, expired or already used code, or malformed body. The enrolment stays pending.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid code or password.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "No enrolment is pending: call `POST /api/v1/user/me/2fa/setup/` first, or the second factor is already active.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not enabled.")
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
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The instance has no `TOTP_ENCRYPTION_KEY`: the second factor is not configured.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[post("/2fa/confirm/")]
pub async fn confirm_two_factor(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    payload: ValidatedJson<TwoFactorCodeView>,
    secrets: Option<web::Data<SecretCipher>>,
    limits: Option<web::Data<RateLimits>>,
) -> Result<impl Responder, TwoFactorError> {
    let secrets = cipher(secrets.as_ref())?;
    spend_budget(limits.as_ref().map(web::Data::get_ref), auth_user.id)?;
    let payload = payload.into_inner();
    let db = state.get_smart_db();
    let record: TotpRecord = match db
        .fetch_one::<TotpRecord, _>(&GetTotpQueryView::new(auth_user.id))
        .await
    {
        Ok(record) if !record.confirmed => record,
        Ok(_) | Err(ApiLibError::Database(DbError::NotFound)) => {
            return Err(TwoFactorError::NotEnabled)
        }
        Err(e) => return Err(db_error("read the pending secret", &e)),
    };
    let sealed = general_purpose::STANDARD
        .decode(&record.secret)
        .map_err(|_| TwoFactorError::DatabaseError)?;
    let secret = secrets.open(&sealed).map_err(|e| {
        tracing::error!("Two-factor confirm: {e}");
        TwoFactorError::DatabaseError
    })?;
    let step = second_factor::verify_code(&secret, payload.code(), second_factor::now_unix(), None)
        .ok_or(TwoFactorError::InvalidCredentials)?;

    let mut tx = db.begin().await.map_err(|e| db_error("begin", &e))?;
    match tx
        .fetch_one::<Written, _>(&ConfirmTotpQueryView::new(
            auth_user.id,
            step,
            &record.secret,
        ))
        .await
    {
        Ok(_) => {}
        Err(ApiLibError::Database(DbError::NotFound)) => return Err(TwoFactorError::NotEnabled),
        Err(e) => return Err(db_error("confirm", &e)),
    }
    let backup_codes = two_factor::issue_backup_codes(&mut tx, auth_user.id).await?;
    tx.commit().await.map_err(|e| db_error("commit", &e))?;
    Ok(HttpResponse::Ok().json(BackupCodesView { backup_codes }))
}

#[utoipa::path(
    post,
    path = "/2fa/disable/",
    summary = "Turn the second factor off",
    description = "Removes the TOTP secret and the backup codes of the caller. Requires the \
                   current password **and** a valid code (application or backup code), so a stolen \
                   session alone cannot weaken the account. A mairie that made the second factor \
                   mandatory (MAIR-808) refuses it.\n\n\
                   Rate limited per user: twenty codes or enrolments per 15 minutes.",
    request_body(
        content = DisableTwoFactorView,
        description = "Current password and a valid code.",
        example = json!({ "password": "MotDePasse!123", "code": "492039" })
    ),
    responses(
        (status = 204, description = "Second factor removed."),
        (
            status = 400,
            description = "Wrong password or wrong code (not told apart), or malformed body. Nothing is removed.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid code or password.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "The second factor is not active.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not enabled.")
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
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The instance has no `TOTP_ENCRYPTION_KEY`: the second factor is not configured.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[post("/2fa/disable/")]
pub async fn disable_two_factor(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    payload: ValidatedJson<DisableTwoFactorView>,
    secrets: Option<web::Data<SecretCipher>>,
    limits: Option<web::Data<RateLimits>>,
) -> Result<impl Responder, TwoFactorError> {
    let secrets = cipher(secrets.as_ref())?;
    spend_budget(limits.as_ref().map(web::Data::get_ref), auth_user.id)?;
    let payload = payload.into_inner();
    let db = state.get_smart_db();
    if !two_factor::is_enabled(db, auth_user.id)
        .await
        .map_err(|e| db_error("status", &e))?
    {
        return Err(TwoFactorError::NotEnabled);
    }
    let stored: UserPassword = db
        .fetch_one(&GetUserPasswordQueryView::new(auth_user.id))
        .await
        .map_err(|e| db_error("read the password", &e))?;
    let password_ok = match stored.password() {
        Some(hash) => is_password_valid(id_to_sql(auth_user.id), payload.password(), hash).await,
        None => false,
    };
    // A wrong password must not spend a code: the agent keeps their backup codes.
    if !password_ok {
        return Err(TwoFactorError::InvalidCredentials);
    }
    two_factor::verify_code(db, secrets, auth_user.id, payload.code()).await?;
    let mut tx = db.begin().await.map_err(|e| db_error("begin", &e))?;
    tx.execute(&DeleteBackupCodesQueryView::new(auth_user.id))
        .await
        .map_err(|e| db_error("delete the backup codes", &e))?;
    tx.execute(&DeleteTotpQueryView::new(auth_user.id))
        .await
        .map_err(|e| db_error("delete the secret", &e))?;
    tx.commit().await.map_err(|e| db_error("commit", &e))?;
    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    path = "/2fa/backup-codes/",
    summary = "Regenerate the backup codes",
    description = "Replaces the backup codes of the caller with ten new ones; the old ones stop \
                   working. Requires a valid code (application or backup code). The new codes are \
                   visible **only in this response**.\n\n\
                   Rate limited per user: twenty codes or enrolments per 15 minutes.",
    request_body(
        content = TwoFactorCodeView,
        description = "A valid code of the application, or an unspent backup code.",
        example = json!({ "code": "492039" })
    ),
    responses(
        (
            status = 200,
            description = "New backup codes, shown once.",
            body = BackupCodesView,
            example = json!({ "backup_codes": ["k3m9-x7pq", "h4wn-2tre", "p8dc-m5zx", "w2ja-9ukb", "n7sy-4qfh", "t6ge-3xvr", "c5mz-8hwd", "y9pk-6nja", "r3fb-7csm", "x4qh-2yte"] })
        ),
        (
            status = 400,
            description = "Wrong or already used code, or malformed body. The former codes stay valid.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid code or password.")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 404,
            description = "The second factor is not active.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not enabled.")
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
            description = "Database error.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The instance has no `TOTP_ENCRYPTION_KEY`: the second factor is not configured.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Users",
    security(("jwt" = []))
)]
#[post("/2fa/backup-codes/")]
pub async fn regenerate_backup_codes(
    state: web::Data<AppState>,
    auth_user: AuthenticatedUser,
    payload: ValidatedJson<TwoFactorCodeView>,
    secrets: Option<web::Data<SecretCipher>>,
    limits: Option<web::Data<RateLimits>>,
) -> Result<impl Responder, TwoFactorError> {
    let secrets = cipher(secrets.as_ref())?;
    spend_budget(limits.as_ref().map(web::Data::get_ref), auth_user.id)?;
    let payload = payload.into_inner();
    let db = state.get_smart_db();
    two_factor::verify_code(db, secrets, auth_user.id, payload.code()).await?;
    let mut tx = db.begin().await.map_err(|e| db_error("begin", &e))?;
    let backup_codes = two_factor::issue_backup_codes(&mut tx, auth_user.id).await?;
    tx.commit().await.map_err(|e| db_error("commit", &e))?;
    Ok(HttpResponse::Ok().json(BackupCodesView { backup_codes }))
}
