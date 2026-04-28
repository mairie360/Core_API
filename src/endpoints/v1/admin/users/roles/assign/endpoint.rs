use crate::endpoints::v1::admin::roles::view::RoleWriteView;

use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum AssignError {
    BadRequest,
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for AssignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            AssignError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
            AssignError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for AssignError {
    fn status_code(&self) -> StatusCode {
        match self {
            AssignError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            AssignError::NotFound => StatusCode::NOT_FOUND,
            AssignError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    post,
    path = "/",
    request_body = RoleWriteView,
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("userId" = String, Path, description = "ID de l'utilisateur")
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn assign(_state: web::Data<AppState>) -> Result<impl Responder, AssignError> {
    Ok(HttpResponse::Ok())
}
