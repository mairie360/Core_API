use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserError::InvalidData => write!(f, "Invalid data provided"),
            DeleteUserError::UserAlreadyExists => write!(f, "User already exists"),
            DeleteUserError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserError::InvalidData => StatusCode::BAD_REQUEST,
            DeleteUserError::UserAlreadyExists => StatusCode::CONFLICT,
            DeleteUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user(state: web::Data<AppState>, user_id: u64) -> Result<(), DeleteUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(DeleteUserError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 400, description = "Invalid data provided"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[delete("/")]
pub async fn delete(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, DeleteUserError> {
    delete_user(state, path.into_inner()).await?;

    Ok(HttpResponse::NoContent().body("User deleted successfully!"))
}
