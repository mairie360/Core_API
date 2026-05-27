use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

use crate::database::users::delete_user::{delete_user_query, DeleteUserQueryView};

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserError {
    AlreadyDeleted,
    DatabaseError,
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserError::AlreadyDeleted => write!(f, "User is already deleted"),
            DeleteUserError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserError::AlreadyDeleted => StatusCode::OK,
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

    let view = DeleteUserQueryView::new(user_id);
    delete_user_query(view, pool).await.map_err(|e| {
        eprintln!("Error: {}", e);
        DeleteUserError::AlreadyDeleted
    })?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    responses(
        (status = 200, description = "User is already deleted"),
        (status = 204, description = "User deleted successfully"),
        (status = 400, description = "Bad request"),
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

    Ok(HttpResponse::NoContent())
}
