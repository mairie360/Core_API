use crate::client_ip::client_ip;
use crate::database::auth::unset_first_connection::UnsetFirstConnectionQueryView;
use crate::endpoints::v1::auth::force_change_password::view::ForceChangePasswordView;
use crate::endpoints::validation::ValidatedJson;
use crate::rate_limit::{ip_key, too_many_requests_response, RateLimits};
use crate::session_revocation::{publish_revoked_sessions, revoke_all_user_sessions_in};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::password::hash_password;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
enum ForceChanhePasswordError {
    DatabaseError,
    Forbidden,
    TooManyRequests(u64),
    Unauthorized,
}

impl std::fmt::Display for ForceChanhePasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Forbidden => {
                write!(f, "Unknown user token")
            }
            Self::TooManyRequests(_) => write!(f, "Too many requests, please try again later."),
            Self::Unauthorized => {
                write!(f, "Unauthorized")
            }
        }
    }
}

impl ResponseError for ForceChanhePasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::TooManyRequests(retry_after) = self {
            return too_many_requests_response(*retry_after);
        }
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_id(state: &AppState, token: &str) -> Option<u64> {
    match state
        .get_redis()
        .secure_get::<String>(&format!("{token}/first_connection_id"))
        .await
    {
        Ok(Some(id)) => id.parse().ok(),
        _ => None,
    }
}

/// Sets the new password and revokes the previous sessions in one transaction (MAIR-420). The
/// update itself checks that the first connection is still pending, so two requests racing
/// with the same token cannot both change the password.
async fn change_password(
    smart_db: &SmartDatabase,
    user_id: u64,
    new_password: &str,
) -> Result<Vec<Uuid>, ForceChanhePasswordError> {
    let hashed_password = hash_password(new_password).map_err(|e| {
        tracing::error!("Password hashing error: {e}");
        ForceChanhePasswordError::DatabaseError
    })?;
    let database_error = |e: ApiLibError| {
        tracing::error!("Force change password DB Error: {e}");
        ForceChanhePasswordError::DatabaseError
    };
    let mut tx = smart_db.begin().await.map_err(database_error)?;
    let changed: i64 = tx
        .fetch_scalar(&UnsetFirstConnectionQueryView::pending_only(
            user_id,
            &hashed_password,
        ))
        .await
        .map_err(database_error)?;
    if changed == 0 {
        return Err(ForceChanhePasswordError::Unauthorized);
    }
    // A password change ends the sessions opened with the previous password, in every API
    // (MAIR-264). Usually none: a first-connection login opens no session.
    let revoked = revoke_all_user_sessions_in(&mut tx, user_id)
        .await
        .map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(revoked)
}

async fn force_change_password_trigger(
    state: web::Data<AppState>,
    view: ForceChangePasswordView,
) -> Result<(), ForceChanhePasswordError> {
    let Some(user_id) = get_user_id(&state, view.token()).await else {
        return Err(ForceChanhePasswordError::Forbidden);
    };

    let revoked = change_password(state.get_smart_db(), user_id, view.new_password()).await?;
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    consume_first_connection_token(&state, view.token(), user_id).await;

    Ok(())
}

/// Le jeton de première connexion est à usage unique : une fois le mot de passe enregistré, les deux
/// clés posées au login sont supprimées. Un échec Redis n'annule pas le changement déjà persisté.
async fn consume_first_connection_token(state: &AppState, token: &str, user_id: u64) {
    let redis = state.get_redis();
    for key in [
        format!("{token}/first_connection_id"),
        format!("{user_id}/first_connection_token"),
    ] {
        if let Err(error) = redis.secure_delete(&key).await {
            tracing::warn!("Could not delete the first-connection token: {error:?}");
        }
    }
}

#[utoipa::path(
    post,
    path = "",
    summary = "Change the password imposed at the first connection",
    description = "Ends the first-connection flow: `POST /api/v1/auth/login` answers `412` with a \
                   one-time token when the correct initial password is given and the user has not \
                   chosen their own password yet. This endpoint consumes that token and stores the \
                   new password; the user can then log in normally.\n\n\
                   Unlike `reset_password`, no session is opened here: the response body is empty \
                   and `POST /api/v1/auth/login` must be called again.\n\n\
                   Public route: `JwtMiddleware` lets everything under `/auth` through. Rate \
                   limited together with `reset_password`: 10 requests per 15 minutes per client \
                   address, `429` beyond.",
    request_body(
        content = ForceChangePasswordView,
        description = "First-connection token returned by the login `412`, and the new password.",
        example = json!({
            "token": "2f9a1c74-5b3e-4d21-9c8a-7e6f0b1d4a35",
            "new_password": "NouveauMotDePasse!123"
        })
    ),
    responses(
        (
            status = 200,
            description = "Password stored and first-connection token consumed. Empty body.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, `token` longer than 512 characters or containing a control character, or `new_password` shorter than 8 characters, longer than 255 characters or containing a control character.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `new_password`: must be at least 8 characters")
        ),
        (
            status = 401,
            description = "The token is valid but the account is no longer in its first connection: the password was already changed.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized")
        ),
        (
            status = 403,
            description = "Unknown, expired or already consumed first-connection token.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user token")
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
            description = "Database failure while storing the password.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Auth"
)]
#[post("/force_change_password")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn force_change_password(
    state: web::Data<AppState>,
    body: ValidatedJson<ForceChangePasswordView>,
    limits: Option<web::Data<RateLimits>>,
    request: HttpRequest,
) -> Result<impl Responder, ForceChanhePasswordError> {
    if let Some(limits) = limits {
        limits
            .password_token_per_ip
            .hit(&ip_key(client_ip(&request)))
            .map_err(|refused| {
                ForceChanhePasswordError::TooManyRequests(refused.retry_after_seconds)
            })?;
    }
    force_change_password_trigger(state, body.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
