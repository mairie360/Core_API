use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

use crate::client_ip::client_ip;
use crate::database::sessions::get_active_session_by_token::ActiveSession;
use crate::database::sessions::rotate_refresh_token::RotateRefreshTokenQueryView;
use crate::endpoints::v1::sessions::refresh::request_view::RefreshRequestView;
use crate::endpoints::v1::sessions::refresh::response_view::RefreshResponseView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::refresh_token;
use crate::session_jwt::generate_session_jwt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshError {
    DatabaseError,
    InvalidToken,
    TooManyRequests(u64),
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "Session not found"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
        }
    }
}

impl ResponseError for RefreshError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
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
) -> Result<(String, String), RefreshError> {
    // The JWT is typically expired at this point: the user is identified by the refresh token,
    // not by an `AuthenticatedUser`. The token is single use: it is replaced in the same
    // statement that finds its session, so a stolen token stops working as soon as either party
    // refreshes (MAIR-390).
    let new_refresh_token = refresh_token::generate();
    let db_view = RotateRefreshTokenQueryView::new(
        &refresh_token::hash(&view.refresh_token()),
        &refresh_token::hash(&new_refresh_token),
    );

    let session: ActiveSession = match state.get_smart_db().fetch_one(&db_view).await {
        Ok(session) => session,
        Err(ApiLibError::Database(DbError::NotFound)) => return Err(RefreshError::InvalidToken),
        Err(e) => {
            eprintln!("Refresh DB Error: {e}");
            return Err(RefreshError::DatabaseError);
        }
    };

    let user_id = u64::try_from(session.user_id()).map_err(|_| RefreshError::InvalidToken)?;
    // The new JWT stays bound to the same session (`sid` claim), see `session_jwt`.
    let jwt = generate_session_jwt(user_id, session.id()).map_err(|e| {
        eprintln!("JWT Generation Error: {e}");
        RefreshError::DatabaseError
    })?;
    Ok((jwt, new_refresh_token))
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
                   No JWT is needed: the route is served outside the JWT middleware, so an \
                   expired JWT can be renewed.\n\n\
                   **The refresh token is rotated**: the token sent in the request stops working \
                   as soon as this call succeeds, and the next refresh (or `/sessions/revoke`) \
                   must use the one returned here. Presenting an already used token answers \
                   `401`, as do two concurrent refreshes with the same token for the loser. The \
                   session itself, its id and its expiry are unchanged.\n\n\
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
            description = "Unknown, revoked, expired or already rotated refresh token.",
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
    let (new_jwt, new_refresh_token) = refresh_request(body.into_inner(), state).await?;
    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {new_jwt}")))
        .json(RefreshResponseView::new(new_refresh_token)))
}
