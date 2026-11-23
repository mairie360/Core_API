use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

use crate::client_ip::client_ip;
use crate::database::sessions::find_reused_refresh_token::{
    FindReusedRefreshTokenQueryView, ReusedRefreshToken,
};
use crate::database::sessions::get_active_session_by_token::ActiveSession;
use crate::database::sessions::rotate_refresh_token::RotateRefreshTokenQueryView;
use crate::endpoints::v1::sessions::refresh::request_view::RefreshRequestView;
use crate::endpoints::v1::sessions::refresh::response_view::RefreshResponseView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::refresh_token;
use crate::session_jwt::{generate_session_jwt, publish_active_token, user_role_name};
use crate::session_revocation::publish_revoked_sessions;
use uuid::Uuid;

/// Seconds during which the refresh token replaced by a rotation is only refused.
///
/// Several fronts of the same browser share the session cookie and may renew it at the same time:
/// the losers present the token the winner has just replaced. Beyond this window, the replaced
/// token coming back means someone else holds it, and the whole session is revoked.
pub const REFRESH_REUSE_GRACE_SECONDS: i64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshError {
    DatabaseError,
    InvalidToken,
    TooManyRequests(u64),
    /// The new JWT could not be published to Redis: the rotation is rolled back (MAIR-772).
    SessionStoreUnavailable,
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "Session not found"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::SessionStoreUnavailable => write!(
                f,
                "The session store is unavailable: the refresh token is unchanged, retry later."
            ),
        }
    }
}

impl ResponseError for RefreshError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::SessionStoreUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Rotates the refresh token and issues a JWT for its session: returns `(jwt, new refresh token)`.
async fn refresh_request(
    view: RefreshRequestView,
    state: web::Data<AppState>,
    ip_address: std::net::IpAddr,
) -> Result<(String, String), RefreshError> {
    // The JWT is typically expired at this point: the user is identified by the refresh token,
    // not by an `AuthenticatedUser`. The token is single use: it is replaced in the same
    // statement that finds its session, so a stolen token stops working as soon as either party
    // refreshes (MAIR-390).
    let new_refresh_token = refresh_token::generate();
    let current_hash = refresh_token::hash(&view.refresh_token());
    let db_view =
        RotateRefreshTokenQueryView::new(&current_hash, &refresh_token::hash(&new_refresh_token));

    // One transaction: the rotation is only kept once the new JWT is known to Redis (MAIR-772), so a
    // refresh that fails there leaves the presented refresh token valid.
    let mut tx = state.get_smart_db().begin().await.map_err(|e| {
        tracing::error!("Refresh DB Error: {e}");
        RefreshError::DatabaseError
    })?;
    let session: ActiveSession = match tx.fetch_one(&db_view).await {
        Ok(session) => session,
        Err(ApiLibError::Database(DbError::NotFound)) => {
            drop(tx);
            handle_reuse(&state, &current_hash).await?;
            return Err(RefreshError::InvalidToken);
        }
        Err(e) => {
            tracing::error!("Refresh DB Error: {e}");
            return Err(RefreshError::DatabaseError);
        }
    };

    let user_id = u64::try_from(session.user_id()).map_err(|_| RefreshError::InvalidToken)?;
    // The new JWT stays bound to the same session (`sid` claim), see `session_jwt`. The roles are
    // read again: a role granted or removed since the last JWT shows from now on (MAIR-529).
    let role = user_role_name(&state, user_id).await.map_err(|e| {
        tracing::error!("Get user roles DB Error: {e}");
        RefreshError::DatabaseError
    })?;
    let jwt = generate_session_jwt(user_id, session.id(), &role).map_err(|e| {
        tracing::error!("JWT Generation Error: {e}");
        RefreshError::DatabaseError
    })?;
    publish_active_token(&state, user_id, session.id(), &jwt, Some(ip_address))
        .await
        .map_err(|_| RefreshError::SessionStoreUnavailable)?;
    tx.commit().await.map_err(|e| {
        tracing::error!("Refresh DB Error: {e}");
        RefreshError::DatabaseError
    })?;
    Ok((jwt, new_refresh_token))
}

/// Handles a refresh token that matches no current token: when it is the token the
/// last rotation of a session replaced, presented after [`REFRESH_REUSE_GRACE_SECONDS`], the
/// session is revoked (Postgres, then the Redis revocation list) and an `audit` event is logged.
/// The caller answers `401` in every case, so the answer never tells a reuse from an unknown token.
async fn handle_reuse(state: &AppState, token_hash: &str) -> Result<(), RefreshError> {
    let reused: ReusedRefreshToken = match state
        .get_smart_db()
        .fetch_one(&FindReusedRefreshTokenQueryView::new(
            token_hash,
            REFRESH_REUSE_GRACE_SECONDS,
        ))
        .await
    {
        Ok(reused) => reused,
        Err(ApiLibError::Database(DbError::NotFound)) => return Ok(()),
        Err(e) => {
            tracing::error!("Refresh token reuse lookup DB Error: {e}");
            return Err(RefreshError::DatabaseError);
        }
    };
    if !reused.revoked() {
        tracing::info!(
            session_id = %reused.id(),
            "Previous refresh token presented within the grace window, refused without revocation"
        );
        return Ok(());
    }
    // The token itself is never logged, only the session it belongs to.
    tracing::warn!(
        target: "audit",
        event = "refresh_token_reuse",
        session_id = %reused.id(),
        user_id = reused.user_id(),
        session_was_active = reused.was_active(),
        "Refresh token reused after its rotation: session revoked"
    );
    // Other APIs refuse the session's JWTs only through the shared revocation list: a short
    // Redis outage must not leave them accepting a stolen session, so the write is retried.
    // Core already refuses the session through `session_guard` either way.
    publish_with_retry(state, reused.id()).await;
    Ok(())
}

/// Waits before each new attempt of [`publish_with_retry`]: about 1.4 s in all.
const REVOCATION_RETRY_DELAYS: [std::time::Duration; 3] = [
    std::time::Duration::from_millis(100),
    std::time::Duration::from_millis(300),
    std::time::Duration::from_secs(1),
];

/// Publishes the revoked session to Redis, retried after each of [`REVOCATION_RETRY_DELAYS`].
/// Returns whether the key was written; a final failure is logged as critical.
async fn publish_with_retry(state: &AppState, session_id: Uuid) -> bool {
    if publish_revoked_sessions(state.get_redis(), &[session_id]).await {
        return true;
    }
    for delay in REVOCATION_RETRY_DELAYS {
        tokio::time::sleep(delay).await;
        if publish_revoked_sessions(state.get_redis(), &[session_id]).await {
            return true;
        }
    }
    tracing::error!(
        target: "audit",
        event = "refresh_token_reuse_unpublished",
        session_id = %session_id,
        "[CRITICAL] Reused refresh token: session revoked in Postgres but not on the Redis revocation list"
    );
    false
}

/// Registered outside the `/api` scope guarded by `JwtMiddleware` (see `sessions::public_config`):
/// an expired JWT must not prevent getting a new one.
#[utoipa::path(
    post,
    path = "refresh",
    summary = "Renew the JWT",
    description = "Exchanges a valid refresh token for a new JWT and a new refresh token, without \
                   asking for the password again. The new JWT is returned in the `Authorization` \
                   header, the new refresh token in the JSON body.\n\n\
                   The new JWT carries the same claims as the one issued at login, with the \
                   user's roles (`roles`, `role`) read again: a role granted or removed since the \
                   previous JWT shows from this call on.\n\n\
                   No JWT is needed: the route is served outside the JWT middleware, so an \
                   expired JWT can be renewed.\n\n\
                   **The refresh token is rotated**: the token sent in the request stops working \
                   as soon as this call succeeds, and the next refresh (or `/sessions/revoke`) \
                   must use the one returned here. Presenting an already used token answers \
                   `401`, as do two concurrent refreshes with the same token for the loser. The \
                   session itself, its id and its expiry are unchanged.\n\n\
                   **Reuse detection**: the token replaced by the last rotation is \
                   only refused (`401`) during the 10 seconds after that rotation (concurrent \
                   renewals of several fronts). Presented later, it means another party holds \
                   the session's tokens: the whole session is revoked, the current refresh token \
                   and the JWTs included (also through the Redis revocation list), an `audit` \
                   event is logged, and the answer is the same `401`.\n\n\
                   Rate limited: 60 requests per minute per client address, `429` beyond.",
    request_body(
        content = RefreshRequestView,
        description = "Refresh token obtained at login or by the previous refresh.",
        example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
    ),
    responses(
        (
            status = 200,
            description = "New JWT issued in the `Authorization` header; the body holds the refresh token that replaces the one sent.",
            body = RefreshResponseView,
            headers(
                ("Authorization" = String, description = "New access JWT, prefixed with `Bearer `.")
            ),
            example = json!({ "refresh_token": "example-rotated-refresh-token" })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing `refresh_token`, or `refresh_token` longer than 512 characters or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `refresh_token`: must not contain control characters")
        ),
        (
            status = 401,
            description = "Unknown, revoked, expired or already rotated refresh token. The token replaced by the latest rotation, presented more than 10 seconds after it, also revokes its session; an older token is only refused.",
            body = String,
            content_type = "text/plain",
            example = json!("Session not found")
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
            description = "Database failure, or the new JWT could not be generated.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
        (
            status = 503,
            description = "The new JWT could not be published to the session store (Redis): nothing is changed, the refresh token sent is still valid. Retry later.",
            body = String,
            content_type = "text/plain",
            example = json!("The session store is unavailable: the refresh token is unchanged, retry later.")
        )
    ),
    tag = "Sessions"
)]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn refresh(
    body: ValidatedJson<RefreshRequestView>,
    state: web::Data<AppState>,
    limits: Option<web::Data<RateLimits>>,
    request: HttpRequest,
) -> Result<impl Responder, RefreshError> {
    if let Some(limits) = limits {
        limits
            .refresh_per_ip
            .hit(&ip_key(client_ip(&request)))
            .map_err(|refused| RefreshError::TooManyRequests(refused.retry_after_seconds))?;
    }
    let (new_jwt, new_refresh_token) =
        refresh_request(body.into_inner(), state, client_ip(&request)).await?;
    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {new_jwt}")))
        .json(RefreshResponseView::new(new_refresh_token)))
}
