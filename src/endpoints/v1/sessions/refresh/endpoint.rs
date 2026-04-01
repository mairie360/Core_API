use actix_web::http::StatusCode;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::queries::is_session_token_valid_query;
use mairie360_api_lib::database::query_views::IsSessionTokenValidQueryView;
use mairie360_api_lib::jwt_manager::generate_jwt;
use mairie360_api_lib::pool::AppState;

use crate::endpoints::v1::sessions::refresh::request_view::RefreshRequestView;
use crate::endpoints::AuthenticatedUser;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq)]
enum RefreshError {
    InvalidToken,
    DatabaseError,
}

impl std::fmt::Display for RefreshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefreshError::InvalidToken => write!(f, "User not found."),
            RefreshError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for RefreshError {
    fn status_code(&self) -> StatusCode {
        match self {
            RefreshError::InvalidToken => StatusCode::UNAUTHORIZED, // On garde 401 selon tes specs
            RefreshError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn refresh_request(
    user: AuthenticatedUser,
    ip_adress: IpAddr,
    view: RefreshRequestView,
    state: web::Data<AppState>,
) -> Result<String, RefreshError> {
    let user_id = user.id;

    let db_view = IsSessionTokenValidQueryView::new(user_id, view.refresh_token, ip_adress);

    let is_valid = is_session_token_valid_query(db_view, state.db_pool.clone()).await;

    match is_valid {
        Ok(true) => generate_jwt(&user_id.to_string()).map_err(|_| RefreshError::DatabaseError),
        Ok(false) => return Err(RefreshError::InvalidToken),
        Err(_) => return Err(RefreshError::DatabaseError),
    }
}

#[utoipa::path(
    post,
    path = "refresh",
    request_body = RefreshRequestView,
    responses(
        (status = 200, description = "Token refreshed successfully"),
        (status = 403, description = "Invalid token"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
)]
#[post("/refresh")]
pub async fn refresh(
    user: AuthenticatedUser,
    body: web::Json<RefreshRequestView>,
    request: HttpRequest,
    state: web::Data<AppState>,
) -> Result<impl Responder, RefreshError> {
    let view = body.into_inner();

    let ip_adress = request
        .connection_info()
        .realip_remote_addr()
        .unwrap()
        .parse()
        .unwrap();

    let new_jwt = refresh_request(user, ip_adress, view, state).await?;

    Ok(HttpResponse::Ok()
        .append_header(("Authorization", format!("Bearer {}", new_jwt)))
        .body("JWT refreshed successfully"))
}
