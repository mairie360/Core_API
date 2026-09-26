use crate::database::sessions::get_active_session_by_token::{
    ActiveSession, GetActiveSessionByTokenQueryView,
};
use crate::database::sessions::revoke_session_by_token::RevokeSessionByTokenQueryView;
use crate::endpoints::v1::sessions::revoke::request_view::RevokeRequestView;
use crate::session_revocation::publish_revoked_sessions;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};

use crate::endpoints::validation::ValidatedJson;
use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum RevokeError {
    InvalidToken,
    DatabaseError,
}

impl std::fmt::Display for RevokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::InvalidToken => {
                write!(f, "Session not found.")
            }
        }
    }
}

impl ResponseError for RevokeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn revoke_request(
    user: AuthenticatedUser,
    view: RevokeRequestView,
    state: web::Data<AppState>,
    ip_address: std::net::IpAddr,
) -> Result<(), RevokeError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token(), ip_address);

    let is_valid: Result<bool, _> = state.get_smart_db().fetch_scalar(&db_view).await;

    let db_view = match is_valid {
        Ok(true) => RevokeSessionByTokenQueryView::new(user_id, &view.refresh_token()),
        Ok(false) => return Err(RevokeError::InvalidToken),
        Err(_) => return Err(RevokeError::DatabaseError),
    };

    // Session id, to publish the revocation to the other APIs (MAIR-264).
    let session: Option<ActiveSession> = state
        .get_smart_db()
        .fetch_one(&GetActiveSessionByTokenQueryView::new(
            &view.refresh_token(),
        ))
        .await
        .ok();

    state
        .get_smart_db()
        .execute(db_view)
        .await
        .map_err(|_| RevokeError::DatabaseError)?;

    if let Some(session) = session {
        publish_revoked_sessions(state.get_redis(), &[session.id()]).await;
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "revoke",
    summary = "Revoke one of your sessions",
    description = "Revokes the session identified by its refresh token (for example another \
                   device). The token no longer works with `POST /api/v1/sessions/refresh`, the \
                   session moves to the history with `revoked_at` set, and the JWTs of that \
                   session are refused by Core right away. The session is also written to the \
                   Redis revocation list (`revoked:<session id>`, for `JWT_TIMEOUT`), so the other \
                   APIs refuse its JWTs once they run a `mairie360_api_lib` version that checks it.\n\n\
                   A user can only revoke their own sessions. To end the current session from \
                   its JWT alone, use `POST /api/v1/sessions/logout`.",
    request_body(
        content = RevokeRequestView,
        description = "Jeton de rafraîchissement de la session à révoquer.",
        example = json!({ "refresh_token": "8Xo0Qm2rUu0M9v2YF3sJkQ7bN1pW4dC6hL8zT5aR0eE" })
    ),
    responses(
        (
            status = 200,
            description = "Session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Session revoked successfully")
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
            description = "JWT de la requête absent, invalide ou expiré, ou jeton de rafraîchissement inconnu, déjà révoqué, ou n'appartenant pas à l'utilisateur connecté.",
            body = String,
            content_type = "text/plain",
            example = json!("Session not found.")
        ),
        (
            status = 500,
            description = "Erreur de base de données lors de la révocation.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[post("/revoke")]
// actix-web exécute chaque handler sur un runtime single-threaded par worker : la future
// n'a pas besoin d'être Send même si elle retient un HttpRequest (non-Send) à travers un
// .await, contrairement à ce que suppose ce lint pedantic.
#[allow(clippy::future_not_send)]
pub async fn revoke(
    user: AuthenticatedUser,
    body: ValidatedJson<RevokeRequestView>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RevokeError> {
    let view = body.into_inner();
    // Depuis mairie360_api_lib (MAIR-125), l'IP n'est plus un critère de validité du token : elle
    // n'est lue que pour la signature de la vue, avec le même repli que le login.
    let ip_address = request
        .connection_info()
        .realip_remote_addr()
        .and_then(|ip| ip.parse().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    revoke_request(user, view, state, ip_address)
        .await
        .map(|()| HttpResponse::Ok().body("Session revoked successfully"))
}
