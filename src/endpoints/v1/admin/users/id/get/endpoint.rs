use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUserError::InvalidData => write!(f, "Invalid data provided"),
            GetUserError::UserAlreadyExists => write!(f, "User already exists"),
            GetUserError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUserError::InvalidData => StatusCode::BAD_REQUEST,
            GetUserError::UserAlreadyExists => StatusCode::CONFLICT,
            GetUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user(state: web::Data<AppState>, user_id: u64) -> Result<(), GetUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetUserError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "User retrieved successfully"),
        (status = 400, description = "Invalid data provided"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[get("/")]
pub async fn get(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, GetUserError> {
    get_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().body("User retrieved successfully!"))
}
