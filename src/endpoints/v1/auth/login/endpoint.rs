use super::view::{LoginResponseView, LoginView};
use crate::client_ip::client_ip;
use crate::database::auth::change_password::ChangePasswordQueryView;
use crate::database::auth::login::LoginUserQueryView;
use crate::database::ids::id_from_sql;
use crate::database::sessions::create_session::CreateSessionQueryView;
use crate::endpoints::v1::auth::login::view::LoginFirstConnectionResponseView;
use crate::endpoints::validation::ValidatedJson;
use crate::passwords;
use crate::rate_limit::{email_key, ip_key, too_many_requests_response, RateLimits};
use crate::redis_keys::{set_token, FIRST_CONNECTION_TTL_SECONDS};
use crate::refresh_token;
use crate::session_jwt::generate_session_jwt;
use actix_web::{http::StatusCode, post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::password::{hash_password, is_hashed, verify_password};
use mairie360_api_lib::smart_db::SmartDatabase;
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
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
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
/// Retourne une erreur si la génération du JWT échoue.
pub async fn generate_session(
    user_id: u64,
    device_info: &str,
    ip_adress: std::net::IpAddr,
    state: web::Data<AppState>,
) -> Result<(String, String), LoginError> {
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
    state.get_smart_db().execute(view).await.map_err(|e| {
        tracing::error!("Create Session DB Error: {e}");
        LoginError::DatabaseError
    })?;
    // The role claim stays empty: neither the lib nor Core reads it.
    let jwt = generate_session_jwt(user_id, session_id).map_err(|e| {
        tracing::error!("JWT Generation Error: {e}");
        LoginError::TokenGenerationError
    })?;
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

async fn login_user(
    login_view: &LoginView,
    state: web::Data<AppState>,
    ip_adress: std::net::IpAddr,
) -> Result<(String, String), LoginError> {
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

    // Unknown, archived and passwordless (Keycloak-only) accounts all answer `401`, after the
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

    generate_session(
        id_from_sql(user.user_id()),
        &login_view.device_info(),
        ip_adress,
        state,
    )
    .await
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
                   The password is always checked first: an unknown address, an archived account, \
                   an account without a password (Keycloak only) and a wrong password all answer \
                   the same `401`, in the same time.\n\n\
                   Until the user has chosen their own password, a **correct** password answers \
                   `412` instead of `200`: its JSON body carries a first-connection token to \
                   present to `POST /api/v1/auth/force_change_password`, and no session is \
                   opened. It is the only error status of this operation whose body is JSON and \
                   not plain text.\n\n\
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
            description = "Logged in. The JWT is returned in the `Authorization` header, the refresh token in the body.",
            body = LoginResponseView,
            headers(
                ("Authorization" = String, description = "Access JWT, prefixed with `Bearer `.")
            ),
            example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
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
    request: HttpRequest,
) -> Result<impl Responder, LoginError> {
    let login_view = payload.into_inner();
    let ip_address = client_ip(&request);
    check_rate_limits(
        limits.as_ref().map(web::Data::get_ref),
        ip_address,
        &login_view.email(),
    )?;

    let (jwt, refresh_token) = login_user(&login_view, state, ip_address).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {jwt}")))
        .json(LoginResponseView::from(refresh_token)))
}
