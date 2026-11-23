//! `POST /api/v1/sessions/revoke-others`: the caller ends every session but the one of the request
//! ("Déconnecter les autres appareils", MAIR-823).

use crate::database::sessions::revoke_user_sessions::RevokeUserSessionsQueryView;
use crate::endpoints::db_error;
use crate::session_jwt::current_session_id;
use crate::session_revocation::publish_revoked_sessions;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Answer of `POST /api/v1/sessions/revoke-others`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RevokeOthersResponse {
    /// Number of sessions revoked (0 when the request's session was the only one).
    #[schema(example = 3)]
    pub revoked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevokeOthersError {
    Database,
}

impl std::fmt::Display for RevokeOthersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred while accessing the database.")
    }
}

impl ResponseError for RevokeOthersError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    post,
    path = "revoke-others",
    operation_id = "revoke_other_sessions",
    summary = "End every other session",
    description = "Revokes every active session of the user carried by the JWT except the session \
                   of the request (its `sid`), and answers how many were revoked. They move to \
                   the history, their refresh tokens stop working and their ids are written to the \
                   Redis revocation list (`revoked:<session id>`, for `JWT_TIMEOUT`).\n\n\
                   A JWT issued without session id cannot tell its own session apart: every \
                   active session of the user is revoked then.",
    responses(
        (
            status = 200,
            description = "Sessions revoked.",
            body = RevokeOthersResponse,
            example = json!({ "revoked": 3 })
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
            description = "Database error while revoking the sessions. Safe to retry.",
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
#[post("/revoke-others")]
// actix-web runs each handler on a single-threaded runtime per worker: the future does not need
// to be Send even though it holds an HttpRequest across an .await.
#[allow(clippy::future_not_send)]
pub async fn revoke_other_sessions(
    user: AuthenticatedUser,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RevokeOthersError> {
    let view = current_session_id(&request).map_or_else(
        || RevokeUserSessionsQueryView::all(user.id),
        |sid| RevokeUserSessionsQueryView::all_but(user.id, sid),
    );
    let revoked: Vec<Uuid> = state.get_smart_db().fetch_all(&view).await.map_err(|e| {
        db_error::log("revoke the other sessions", &e);
        RevokeOthersError::Database
    })?;
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    Ok(HttpResponse::Ok().json(RevokeOthersResponse {
        revoked: revoked.len(),
    }))
}
