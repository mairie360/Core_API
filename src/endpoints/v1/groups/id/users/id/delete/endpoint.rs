use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum AddAccessError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for AddAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddAccessError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            AddAccessError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for AddAccessError {
    fn status_code(&self) -> StatusCode {
        match self {
            AddAccessError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            AddAccessError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user_from_group(
    state: web::Data<AppState>,
    group_id: u64,
    user_id: u64,
) -> Result<(), AddAccessError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(AddAccessError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    delete,
    path = "/",
    responses(
        (status = 204, description = "User deleted from group successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn delete(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(u64, u64)>,
) -> Result<impl Responder, AddAccessError> {
    let (group_id, user_id) = path.into_inner();
    delete_user_from_group(state, group_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
