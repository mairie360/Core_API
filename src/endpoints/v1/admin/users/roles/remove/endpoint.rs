use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum DeleteError {
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            DeleteError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            DeleteError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[utoipa::path(
    delete,
    path = "/{roleId}",
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("roleId" = i32, Path, description = "User database id"),
        ("userId" = String, Path, description = "ID de l'utilisateur")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{roleId}")]
pub async fn remove(state: web::Data<AppState>) -> Result<impl Responder, DeleteError> {
    Ok(HttpResponse::Ok())
}
