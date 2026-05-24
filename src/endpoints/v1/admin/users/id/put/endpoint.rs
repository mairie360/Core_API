use actix_web::{error::ResponseError, http::StatusCode, put, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PutUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for PutUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PutUserError::InvalidData => write!(f, "Invalid data provided"),
            PutUserError::UserAlreadyExists => write!(f, "User already exists"),
            PutUserError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for PutUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            PutUserError::InvalidData => StatusCode::BAD_REQUEST,
            PutUserError::UserAlreadyExists => StatusCode::CONFLICT,
            PutUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn put_user(state: web::Data<AppState>, user_id: u64) -> Result<(), PutUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(PutUserError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    put,
    path = "",
    responses(
        (status = 200, description = "User patched successfully"),
        (status = 400, description = "Invalid data provided"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[put("/")]
pub async fn put(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, PutUserError> {
    put_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().body("User patched successfully!"))
}
