use actix_web::{error::ResponseError, http::StatusCode, patch, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for PatchUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchUserError::InvalidData => write!(f, "Invalid data provided"),
            PatchUserError::UserAlreadyExists => write!(f, "User already exists"),
            PatchUserError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for PatchUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchUserError::InvalidData => StatusCode::BAD_REQUEST,
            PatchUserError::UserAlreadyExists => StatusCode::CONFLICT,
            PatchUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn patch_user(state: web::Data<AppState>, user_id: u64) -> Result<(), PatchUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(PatchUserError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    patch,
    path = "",
    responses(
        (status = 200, description = "User patched successfully"),
        (status = 400, description = "Invalid data provided"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[patch("/")]
pub async fn patch(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, PatchUserError> {
    patch_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
