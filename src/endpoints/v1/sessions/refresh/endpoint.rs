use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::state::AppState;

use crate::database::sessions::get_active_session_user_id::GetActiveSessionUserIdQueryView;
use crate::endpoints::v1::sessions::refresh::request_view::RefreshRequestView;

#[derive(Debug, Clone, PartialEq)]
pub enum RefreshError {
    DatabaseError,
    InvalidToken,
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshError::InvalidToken => write!(f, "Session not found"),
            RefreshError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for RefreshError {
    fn status_code(&self) -> StatusCode {
        match self {
            RefreshError::InvalidToken => StatusCode::UNAUTHORIZED,
            RefreshError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn refresh_request(
    view: RefreshRequestView,
    state: web::Data<AppState>,
) -> Result<String, RefreshError> {
    // Le JWT est typiquement expiré à ce stade : l'utilisateur est identifié par son refresh
    // token, pas par un `AuthenticatedUser`.
    let db_view = GetActiveSessionUserIdQueryView::new(&view.refresh_token());

    let user_id: i32 = match state.get_smart_db().fetch_scalar(&db_view).await {
        Ok(user_id) => user_id,
        Err(ApiLibError::Database(DbError::NotFound)) => return Err(RefreshError::InvalidToken),
        Err(e) => {
            eprintln!("Refresh DB Error: {e}");
            return Err(RefreshError::DatabaseError);
        }
    };

    // TODO: cf. login/endpoint.rs::generate_session — rôle non encore exploité par la lib.
    generate_jwt(&user_id.to_string(), "").map_err(|e| {
        eprintln!("JWT Generation Error: {e}");
        RefreshError::DatabaseError
    })
}

/// Enregistré hors du scope `/api` protégé par `JwtMiddleware` (cf. `sessions::public_config`) :
/// un JWT expiré ne doit pas empêcher d'en obtenir un nouveau.
#[utoipa::path(
    post,
    path = "refresh",
    request_body = RefreshRequestView,
    responses(
        (status = 200, description = "Token refreshed successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized, invalid, revoked or expired refresh token"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
)]
pub async fn refresh(
    body: web::Json<RefreshRequestView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, RefreshError> {
    let view = body.into_inner();

    let new_jwt = refresh_request(view, state).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {new_jwt}")))
        .body("JWT refreshed successfully"))
}
