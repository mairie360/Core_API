use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

use crate::endpoints::v1::sessions::admin::refresh::request_view::RefreshPathParamRequestView;

#[derive(Debug, Clone, PartialEq)]
enum AboutError {
    UserNotFound,
    DatabaseError,
}

impl std::fmt::Display for AboutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AboutError::UserNotFound => write!(f, "User not found."),
            AboutError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for AboutError {
    fn status_code(&self) -> StatusCode {
        match self {
            AboutError::UserNotFound => StatusCode::UNAUTHORIZED, // On garde 401 selon tes specs
            AboutError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    post,
    path = "{token_id}/refresh",
    responses(
        (status = 200, description = "Token refreshed successfully"),
        (status = 401, description = "Invalid token ID"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("token_id" = String, Path, description = "The ID of the token"),
    ),
    tag = "Sessions",
    security(("jwt" = [])),
)]
#[post("/{token_id}/refresh")]
pub async fn refresh(
    path: web::Path<RefreshPathParamRequestView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AboutError> {
    let view = path.into_inner();

    Ok(HttpResponse::Ok())
}
