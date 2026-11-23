use super::view::{LoginResponseView, LoginView, PasskeyOfferView};
use crate::client_ip::client_ip;
use crate::database::auth::change_password::ChangePasswordQueryView;
use crate::database::auth::login::LoginUserQueryView;
use crate::database::ids::id_from_sql;
use crate::database::sessions::create_session::CreateSessionQueryView;
use crate::endpoints::v1::auth::login::view::{
    LoginFirstConnectionResponseView, TwoFactorRequiredView, TwoFactorSetupPolicyView,
};
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::rate_limit::{email_key, ip_key, too_many_requests_response, RateLimits};
use crate::redis_keys::{set_token, FIRST_CONNECTION_TTL_SECONDS};
use crate::refresh_token;
use crate::second_factor::SecretCipher;
use crate::session_jwt::{generate_session_jwt, publish_active_token, user_role_name};
use crate::two_factor;
use actix_web::{http::StatusCode, post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::password::{hash_password, is_hashed, verify_password};
use mairie360_api_lib::smart_db::{SmartDatabase, SmartTransaction};
use mairie360_api_lib::state::AppState;
use std::sync::LazyLock;
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq)]
pub enum LoginError {
    DatabaseError,
    FirstConnectError(String),
    InvalidCredentials,
    RedisError,
    TokenGenerationError,
    TooManyRequests(u64),
    /// Password right, second factor to give: the login token of the second step (MAIR-805).
    TwoFactorRequired(String),
    /// Password right, the instance makes the second factor mandatory and the account has none:
    /// the enrolment token, no session (MAIR-808).
    TwoFactorSetupRequired(String, TwoFactorSetupPolicyView),
    /// The account has a second factor but `TOTP_ENCRYPTION_KEY` is not configured: refused rather
    /// than opened without it.
    TwoFactorUnavailable,
}

impl std::fmt::Debug for LoginError {
    // Never the first-connection token: an error's `Debug` ends up in logs and traces (MAIR-290).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => write!(f, "DatabaseError"),
            Self::FirstConnectError(_) => write!(f, "FirstConnectError(<token>)"),
            Self::InvalidCredentials => write!(f, "InvalidCredentials"),
            Self::RedisError => write!(f, "RedisError"),
            Self::TokenGenerationError => write!(f, "TokenGenerationError"),
            Self::TooManyRequests(seconds) => write!(f, "TooManyRequests({seconds})"),
            Self::TwoFactorRequired(_) => write!(f, "TwoFactorRequired(<token>)"),
            Self::TwoFactorSetupRequired(..) => write!(f, "TwoFactorSetupRequired(<token>)"),
            Self::TwoFactorUnavailable => write!(f, "TwoFactorUnavailable"),
        }
    }
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "Invalid credentials provided."),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::TokenGenerationError => write!(f, "Failed to generate JWT token."),
            // Never the token itself: the error message ends up in logs and traces.
            Self::FirstConnectError(_) => {
                write!(f, "First connection: the password must be changed.")
            }
            Self::RedisError => write!(f, "Internal Redis error."),
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::TwoFactorRequired(_) => write!(f, "Second factor required."),
            Self::TwoFactorSetupRequired(..) => write!(f, "Second factor setup required."),
            Self::TwoFactorUnavailable => {
                write!(f, "Two-factor authentication is not configured.")
            }
        }
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError | Self::RedisError | Self::TokenGenerationError => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::FirstConnectError(_) => StatusCode::PRECONDITION_FAILED,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::TwoFactorRequired(_) | Self::TwoFactorSetupRequired(..) => StatusCode::ACCEPTED,
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
        if let Self::TwoFactorSetupRequired(token, policy) = self {
            return HttpResponse::build(self.status_code())
                .json(TwoFactorRequiredView::setup(token.clone(), *policy));
        }
        if let Self::FirstConnectError(token) = self {
            return HttpResponse::build(self.status_code())
                .json(LoginFirstConnectionResponseView::new(token.clone()));
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// # Errors
///
/// Returns an error if the JWT generation fails.
pub async fn generate_session(
    user_id: u64,
    device_info: &str,
    ip_adress: std::net::IpAddr,
    state: web::Data<AppState>,
) -> Result<(String, String), LoginError> {
    let (view, session_id, refresh_token) = new_session(user_id, device_info, ip_adress);
    state.get_smart_db().execute(view).await.map_err(|e| {
        tracing::error!("Create Session DB Error: {e}");
        LoginError::DatabaseError
    })?;
    finish_session(user_id, session_id, refresh_token, ip_adress, &state).await
}

/// Same as [`generate_session`], but the session row is written inside `tx`: the caller commits
/// it together with the checks it made in that transaction (MAIR-805, second step of the login).
///
/// # Errors
///
/// Returns an error if the insert or the JWT generation fails; `tx` must then be dropped.
pub async fn generate_session_in(
    tx: &mut SmartTransaction,
    user_id: u64,
    device_info: &str,
    ip_adress: std::net::IpAddr,
    state: &web::Data<AppState>,
) -> Result<(String, String), LoginError> {
    let (view, session_id, refresh_token) = new_session(user_id, device_info, ip_adress);
    tx.execute(&view).await.map_err(|e| {
        tracing::error!("Create Session DB Error: {e}");
        LoginError::DatabaseError
    })?;
    finish_session(user_id, session_id, refresh_token, ip_adress, state).await
}

fn new_session(
    user_id: u64,
    device_info: &str,
    ip_adress: std::net::IpAddr,
) -> (CreateSessionQueryView, Uuid, String) {
    let refresh_token = refresh_token::generate();
    // The session id is chosen here so the JWT can carry it (`sid` claim): logout revokes exactly
    // this session, and the JWT stops working once it is revoked (MAIR-226). Each login keeps its
    // own session: other devices' sessions stay active.
    let session_id = Uuid::new_v4();
    // Only the digest is stored: the token itself never reaches the database (MAIR-390).
    let view = CreateSessionQueryView::with_id(
        session_id,
        user_id,
        &refresh_token::hash(&refresh_token),
        device_info,
        ip_adress,
    );
    (view, session_id, refresh_token)
}

async fn finish_session(
    user_id: u64,
    session_id: Uuid,
    refresh_token: String,
    ip_address: std::net::IpAddr,
    state: &web::Data<AppState>,
) -> Result<(String, String), LoginError> {
    // The BFFs read the caller's role from the JWT (MAIR-529).
    let role = user_role_name(state, user_id).await.map_err(|e| {
        tracing::error!("Get user roles DB Error: {e}");
        LoginError::DatabaseError
    })?;
    let jwt = generate_session_jwt(user_id, session_id, &role).map_err(|e| {
        tracing::error!("JWT Generation Error: {e}");
        LoginError::TokenGenerationError
    })?;
    publish_active_token(state, user_id, session_id, &jwt, Some(ip_address)).await;
    Ok((jwt, refresh_token))
}

async fn generate_first_connection_token(
    user_id: u64,
    state: web::Data<AppState>,
) -> Result<String, LoginError> {
    let redis = state.get_redis();

    if let Ok(Some(token)) = redis
        .secure_get::<String>(&format!("{user_id}/first_connection_token"))
        .await
    {
        return Ok(token);
    }
    let token = Uuid::new_v4().to_string();
    set_token(
        redis,
        &format!("{user_id}/first_connection_token"),
        &token,
        FIRST_CONNECTION_TTL_SECONDS,
    )
    .await
    .map_err(|e| {
        tracing::error!("Redis Error: {e}");
        LoginError::RedisError
    })?;
    set_token(
        redis,
        &format!("{token}/first_connection_id"),
        &user_id.to_string(),
        FIRST_CONNECTION_TTL_SECONDS,
    )
    .await
    .map_err(|e| {
        tracing::error!("Redis Error: {e}");
        LoginError::RedisError
    })?;
    Ok(token)
}

/// Rehashes a legacy plaintext password into an argon2id hash once its owner has proven they
/// know it. A failure here (hashing or write) is logged and swallowed: the login itself already
/// succeeded and must not fail because the opportunistic migration didn't.
async fn migrate_plaintext_password(smart_db: &SmartDatabase, user_id: u64, plaintext: &str) {
    let hashed = match passwords::hash(plaintext).await {
        Ok(hashed) => hashed,
        Err(e) => {
            tracing::error!("Failed to hash password while migrating user {user_id}: {e}");
            return;
        }
    };
    if let Err(e) = smart_db
        .execute(ChangePasswordQueryView::new(&hashed, user_id))
        .await
    {
        tracing::error!("Failed to persist migrated password for user {user_id}: {e}");
    }
}

/// Argon2id hash of a random value, verified against when the e-mail matches no account (or a
/// passwordless one) so the answer takes as long as for a real account: the response time does
/// not tell which addresses have an account.
static DUMMY_PASSWORD_HASH: LazyLock<Option<String>> =
    LazyLock::new(|| hash_password(&refresh_token::generate()).ok());

async fn burn_password_verification(password: &str) {
    let password = password.to_owned();
    // On the blocking pool like `passwords::verify`, the one-time dummy hash included.
    let _ = web::block(move || {
        if let Some(dummy) = DUMMY_PASSWORD_HASH.as_deref() {
            let _ = verify_password(&password, dummy);
        }
    })
    .await;
}

/// Checks `password` against the stored value, hashed or legacy plaintext.
pub(crate) async fn is_password_valid(user_id: i32, password: &str, stored_password: &str) -> bool {
    // Accounts created before the password migration still hold a plaintext password: compare it
    // directly (the caller then replaces it with a hash). Everything hashed already goes through
    // `verify_password`, which only accepts a value `is_hashed` agrees is an argon2id PHC string.
    if is_hashed(stored_password) {
        passwords::verify(password, stored_password)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to verify the password hash of user {user_id}: {e}");
                false
            })
    } else {
        password == stored_password.trim()
    }
}

/// What a password alone leads to for an account (MAIR-805, MAIR-808).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TwoFactorStep {
    /// The account has a usable second factor: the login token of the second step.
    Code(String),
    /// The instance requires a second factor and the account has no usable one: the enrolment
    /// token, nothing else is allowed.
    Setup(String, TwoFactorSetupPolicyView),
}

impl TwoFactorStep {
    /// The error answered instead of a session.
    pub(crate) fn into_login_error(self) -> LoginError {
        match self {
            Self::Code(token) => LoginError::TwoFactorRequired(token),
            Self::Setup(token, policy) => LoginError::TwoFactorSetupRequired(token, policy),
        }
    }
}

/// Decides what follows a verified password for `user_id`: the second step when the account has a
/// factor the policy still accepts, the mandatory enrolment when the policy requires a factor the
/// account does not have, a session (`None`) otherwise.
///
/// # Errors
///
/// [`LoginError::TwoFactorUnavailable`] when a factor is needed (to check or to enrol) but the
/// instance cannot seal secrets, [`LoginError::DatabaseError`] / [`LoginError::RedisError`] on a
/// failure.
pub(crate) async fn two_factor_challenge(
    state: &AppState,
    user_id: u64,
    device_info: &str,
    can_verify: bool,
) -> Result<Option<TwoFactorStep>, LoginError> {
    let db_error = |e: mairie360_api_lib::error::ApiLibError| {
        tracing::error!("Second factor status DB Error: {e}");
        LoginError::DatabaseError
    };
    let enabled = two_factor::is_enabled(state.get_smart_db(), user_id)
        .await
        .map_err(db_error)?;
    let policy = two_factor::load_policy(state.get_smart_db())
        .await
        .map_err(db_error)?;
    // A factor of a mode the administrator removed does not count.
    let usable = enabled && policy.totp_allowed;
    if !usable && !policy.required {
        return Ok(None);
    }
    if !can_verify {
        tracing::error!("User {user_id} needs a second factor but TOTP_ENCRYPTION_KEY is not set");
        return Err(LoginError::TwoFactorUnavailable);
    }
    let redis_error = |e: String| {
        tracing::error!("Redis Error: {e}");
        LoginError::RedisError
    };
    if usable {
        two_factor::begin_login(state.get_redis(), user_id, device_info)
            .await
            .map(|token| Some(TwoFactorStep::Code(token)))
            .map_err(redis_error)
    } else {
        // `user_totp` may hold the confirmed secret of a removed mode: enrolling starts over.
        two_factor::begin_setup(state.get_redis(), user_id, device_info)
            .await
            .map(|token| {
                Some(TwoFactorStep::Setup(
                    token,
                    TwoFactorSetupPolicyView::from(&policy),
                ))
            })
            .map_err(redis_error)
    }
}

async fn login_user(
    login_view: &LoginView,
    state: web::Data<AppState>,
    ip_adress: std::net::IpAddr,
    can_verify_second_factor: bool,
) -> Result<(String, String, u64), LoginError> {
    let view = LoginUserQueryView::new(login_view.email(), login_view.password());

    let user_record = match state
        .get_smart_db()
        .fetch_one::<crate::database::auth::login::LoginUserQueryResultView, _>(&view)
        .await
    {
        Ok(result) => Some(result),
        Err(ApiLibError::Database(DbError::NotFound)) => None,
        Err(e) => {
            tracing::error!("Login DB Error: {e}");
            return Err(LoginError::DatabaseError);
        }
    };

    // Unknown, archived and passwordless accounts all answer `401`, after the
    // same password verification work as a real account.
    let Some((user, stored_password)) = user_record.and_then(|user| {
        let stored_password = user.password()?.to_string();
        Some((user, stored_password))
    }) else {
        burn_password_verification(&login_view.password()).await;
        return Err(LoginError::InvalidCredentials);
    };

    // The password is checked before anything else, the first-connection token included: knowing
    // the e-mail of a new account must not be enough to choose its password (MAIR-390).
    if !is_password_valid(user.user_id(), &login_view.password(), &stored_password).await {
        tracing::error!(
            "Login failed: invalid credentials for user {}",
            user.user_id()
        );
        return Err(LoginError::InvalidCredentials);
    }

    if !is_hashed(&stored_password) {
        migrate_plaintext_password(
            state.get_smart_db(),
            id_from_sql(user.user_id()),
            &login_view.password(),
        )
        .await;
    }

    if user.first_connect() {
        return Err(LoginError::FirstConnectError(
            generate_first_connection_token(id_from_sql(user.user_id()), state).await?,
        ));
    }

    // A protected account gets no session from its password alone (MAIR-805).
    if let Some(step) = two_factor_challenge(
        &state,
        id_from_sql(user.user_id()),
        &login_view.device_info(),
        can_verify_second_factor,
    )
    .await?
    {
        return Err(step.into_login_error());
    }

    let user_id = id_from_sql(user.user_id());
    let (jwt, refresh_token) =
        generate_session(user_id, &login_view.device_info(), ip_adress, state).await?;
    Ok((jwt, refresh_token, user_id))
}

/// Counts the attempt against the per-address and per-account budgets.
fn check_rate_limits(
    limits: Option<&RateLimits>,
    ip_address: std::net::IpAddr,
    email: &str,
) -> Result<(), LoginError> {
    let Some(limits) = limits else {
        return Ok(());
    };
    limits
        .login_per_ip
        .hit(&ip_key(ip_address))
        .and_then(|()| limits.login_per_email.hit(&email_key(email)))
        .map_err(|refused| LoginError::TooManyRequests(refused.retry_after_seconds))
}

#[utoipa::path(
    post,
    path = "",
    summary = "Log in",
    description = "Authenticates a user by e-mail and password, opens a session and returns a JWT \
                   in the `Authorization` header plus a refresh token in the body. Public route: \
                   `JwtMiddleware` lets everything under `/auth` through.\n\n\
                   The JWT carries the user id (`sub`), the session id (`sid`), its expiry \
                   (`exp`) and the user's roles: `roles` lists every role name ordered by role id \
                   (empty array when the user has none) and `role` repeats the first one (empty \
                   string when none). The roles are read at login and at \
                   `POST /api/v1/sessions/refresh`: a role granted or removed later shows at the \
                   next refresh only.\n\n\
                   The password is always checked first: an unknown address, an archived account, \
                   an account without a password and a wrong password all answer \
                   the same `401`, in the same time.\n\n\
                   Until the user has chosen their own password, a **correct** password answers \
                   `412` instead of `200`: its JSON body carries a first-connection token to \
                   present to `POST /api/v1/auth/force_change_password`, and no session is \
                   opened. It is the only error status of this operation whose body is JSON and \
                   not plain text.\n\n\
                   An account protected by the second factor (MAIR-805) gets **no session** from its \
                   password: the answer is `202` with a login token to give, with a code of the \
                   authenticator application or a backup code, to `POST /api/v1/auth/login/2fa`.\n\n\
                   Rate limited: 30 attempts per minute per client address and 10 per 15 minutes \
                   per e-mail address, successful or not; beyond that the answer is `429`.",
    request_body(
        content = LoginView,
        description = "Credentials of the user and description of the device used.",
        example = json!({
            "email": "jean.dupont@mairie360.fr",
            "password": "MotDePasse!123",
            "device_info": "Chrome 140 on Windows 11"
        })
    ),
    responses(
        (
            status = 200,
            description = "Logged in. The JWT is returned in the `Authorization` header, the refresh token in the body, with `passkey` (whether the login screen should propose a passkey: `allowed` when the instance and the administrator accept passkeys, `registered` when the account already has one).",
            body = LoginResponseView,
            headers(
                ("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")
            ),
            example = json!({ "refresh_token": "example-rotated-refresh-token", "passkey_offer": { "allowed": true, "registered": false } })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, or `email` (320), `password` (255) or `device_info` (512) longer than its limit or containing a control character. A well-formed but unknown address answers `401`, not `400`.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `email`: must not contain control characters")
        ),
        (
            status = 401,
            description = "Unknown e-mail address, archived account, account without a password, or wrong password. The cases are not told apart.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid credentials provided.")
        ),
        (
            status = 202,
            description = "Correct password, no session opened. `2fa_required` (account protected by the second factor): call `POST /api/v1/auth/login/2fa` with the token within five minutes. `2fa_setup_required` (the instance makes the second factor mandatory and the account has none): enrol with `POST /api/v1/auth/2fa/setup` then `confirm`; `policy` tells which modes the administrator allows and whether backup codes are, so the screen offers nothing else. `policy` is absent from `2fa_required`.",
            body = TwoFactorRequiredView,
            example = json!({ "status": "2fa_setup_required", "token": "example-two-factor-token", "policy": { "totp_allowed": true, "passkey_allowed": false, "backup_codes_allowed": true } })
        ),
        (
            status = 412,
            description = "First connection, with the correct password: the password must be changed through `/api/v1/auth/force_change_password` with the returned token. No session is opened.",
            body = LoginFirstConnectionResponseView,
            example = json!({ "token": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35" })
        ),
        (
            status = 429,
            description = "Too many attempts from this client address or for this e-mail address. The `Retry-After` header gives the number of seconds to wait.",
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
            description = "The account has a second factor but the instance has no `TOTP_ENCRYPTION_KEY`: the login is refused rather than opened without it.",
            body = String,
            content_type = "text/plain",
            example = json!("Two-factor authentication is not configured.")
        )
    ),
    tag = "Auth"
)]
#[post("/login")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn login(
    payload: ValidatedJson<LoginView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    secrets: Option<web::Data<SecretCipher>>,
    relying_party: Option<web::Data<crate::passkeys::Passkeys>>,
    emails: Option<web::Data<crate::emails::AccountEmails>>,
    request: HttpRequest,
) -> Result<impl Responder, LoginError> {
    let login_view = payload.into_inner();
    let ip_address = client_ip(&request);
    check_rate_limits(
        limits.as_ref().map(web::Data::get_ref),
        ip_address,
        &login_view.email(),
    )?;

    let (jwt, refresh_token, user_id) =
        login_user(&login_view, state.clone(), ip_address, secrets.is_some()).await?;
    crate::login_alert::spawn(
        &state,
        emails.as_ref(),
        user_id,
        &login_view.device_info(),
        ip_address,
    );
    let offer = passkey_offer(&state, relying_party.is_some(), user_id).await;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token).with_passkey(offer)))
}

/// What the login screen needs to propose a passkey after a password sign-in. A failure is logged and
/// reads as "nothing to propose": it never blocks the sign-in.
pub(crate) async fn passkey_offer(
    state: &AppState,
    configured: bool,
    user_id: u64,
) -> PasskeyOfferView {
    use crate::database::passkeys::{ListPasskeysQueryView, PasskeyRecord};
    let none = PasskeyOfferView {
        allowed: false,
        registered: false,
    };
    if !configured {
        return none;
    }
    let db = state.get_smart_db();
    let allowed = match crate::two_factor::load_policy(db).await {
        Ok(policy) => policy.passkey_allowed,
        Err(e) => {
            tracing::warn!("Passkey proposal: the policy is unreadable: {e}");
            return none;
        }
    };
    let registered = match db
        .fetch_all::<PasskeyRecord, _>(&ListPasskeysQueryView::new(user_id))
        .await
    {
        Ok(rows) => !rows.is_empty(),
        Err(e) => {
            tracing::warn!(
                user_id,
                "Passkey proposal: the passkeys are unreadable: {e}"
            );
            return none;
        }
    };
    PasskeyOfferView {
        allowed,
        registered,
    }
}
