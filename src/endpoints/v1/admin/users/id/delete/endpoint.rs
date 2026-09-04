use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::state::AppState;

use crate::database::users::delete_user::DeleteUserQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserError {
    AlreadyDeleted,
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteUserError::AlreadyDeleted => write!(f, "User is already deleted"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteUserError::AlreadyDeleted => StatusCode::OK,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user(state: web::Data<AppState>, user_id: u64) -> Result<(), DeleteUserError> {
    let view = DeleteUserQueryView::new(user_id);
    state.get_smart_db().execute(view).await.map_err(|e| {
        eprintln!("Error: {}", e);
        DeleteUserError::AlreadyDeleted
    })?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    params(
        ("userId" = u64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "User is already deleted"),
        (status = 204, description = "User deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[delete("/")]
pub async fn admin_delete_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, DeleteUserError> {
    delete_user(state, path.into_inner()).await?;

    Ok(HttpResponse::NoContent())
}
