use crate::endpoints::v1::admin::sessions::audit::request_view::AuditPathParamRequestView;
use crate::endpoints::v1::admin::sessions::audit::response_view::AuditResponseView;

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
    path = "{user_id}/audit",
    responses(
        (status = 200, description = "User info retrieved successfully", body = AuditResponseView),
        (status = 401, description = "Invalid user ID"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("user_id" = u64, Path, description = "The ID of the user"),
    ),
    tag = "Admin",
    security(
        ("jwt" = [])
    )
)]
#[get("/{user_id}/audit")]
pub async fn audit(
    path: web::Path<AuditPathParamRequestView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AboutError> {
    let view = path.into_inner();

    Ok(HttpResponse::Ok())
}
