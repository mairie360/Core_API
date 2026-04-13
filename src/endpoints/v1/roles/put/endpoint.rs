use crate::endpoints::v1::roles::put::view::PutView;
use crate::endpoints::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{put, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchError {
    BadRequest,
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            PatchError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
            PatchError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for PatchError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            PatchError::NotFound => StatusCode::NOT_FOUND,
            PatchError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    put,
    path = "/{id}",
    request_body = PutView,
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = i32, Path, description = "Role database id") // <--- AJOUTE CECI
    ),
    tag = "Roles",
    security(
        ("jwt" = [])
    )
)]
#[put("/{id}")]
pub async fn put(state: web::Data<AppState>) -> Result<impl Responder, PatchError> {
    Ok(HttpResponse::Ok())
}
