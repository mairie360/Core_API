use crate::database::roles::can_delete_role::{can_delete_role_query, CanDeleteRoleQueryView};
use crate::database::roles::delete_role::{delete_role_query, DeleteRoleQueryView};
use crate::database::roles::does_role_exist::{does_role_exist_query, DoesRoleExistQueryView};
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq)]
enum DeleteError {
    Forbidden,
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
            DeleteError::Forbidden => {
                write!(f, "The requested resource cannot be deleted.")
            }
        }
    }
}

impl ResponseError for DeleteError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            DeleteError::NotFound => StatusCode::NOT_FOUND,
            DeleteError::Forbidden => StatusCode::FORBIDDEN,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn does_role_exist(id: u64, pool: PgPool) -> bool {
    let view = DoesRoleExistQueryView::new(id);
    let result = does_role_exist_query(view, pool).await;
    result.unwrap()
}

async fn can_delete_role(id: u64, pool: PgPool) -> bool {
    let view = CanDeleteRoleQueryView::new(id);
    let result = can_delete_role_query(view, pool).await;
    result.unwrap()
}

async fn delete_role(id: u64, state: web::Data<AppState>) -> Result<(), DeleteError> {
    if !does_role_exist(id, state.db_pool.clone()).await {
        return Err(DeleteError::NotFound);
    }
    if !can_delete_role(id, state.db_pool.clone()).await {
        return Err(DeleteError::Forbidden);
    }
    let view = DeleteRoleQueryView::new(id);
    let result = delete_role_query(view, state.db_pool.clone()).await;
    result.map_err(|_| DeleteError::DatabaseError)
}

#[utoipa::path(
    delete,
    path = "/{id}",
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 403, description = "Role cannot be deleted"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = i32, Path, description = "Role database id")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{id}")]
pub async fn delete(
    id: web::Path<u64>,
    state: web::Data<AppState>,
) -> Result<impl Responder, DeleteError> {
    delete_role(id.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
