use crate::endpoints::v1::sessions::history::response_view::HistoryResponseView;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
// use serde_json;

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
    get,
    path = "history",
    responses(
        (status = 200, description = "User info retrieved successfully", body = HistoryResponseView),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[get("/history")]
pub async fn history(state: web::Data<AppState>) -> Result<impl Responder, AboutError> {
    Ok(HttpResponse::Ok())
}
