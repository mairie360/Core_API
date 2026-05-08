use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum DeleteGroupError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for DeleteGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            DeleteGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for DeleteGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            DeleteGroupError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    id: u64,
) -> Result<(), DeleteGroupError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(DeleteGroupError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    responses(
        (status = 204, description = "Group deleted successfully"),
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
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
) -> Result<impl Responder, DeleteGroupError> {
    delete_group(user, state, id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
