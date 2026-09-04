use crate::database::roles::can_delete_role::CanDeleteRoleQueryView;
use crate::database::roles::delete_role::DeleteRoleQueryView;
use crate::database::roles::does_role_exist::DoesRoleExistQueryView;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

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

async fn does_role_exist(id: u64, smart_db: &SmartDatabase) -> bool {
    let view = DoesRoleExistQueryView::new(id);
    smart_db.fetch_scalar(&view).await.unwrap()
}

async fn can_delete_role(id: u64, smart_db: &SmartDatabase) -> bool {
    let view = CanDeleteRoleQueryView::new(id);
    smart_db.fetch_scalar(&view).await.unwrap()
}

async fn delete_role(id: u64, state: web::Data<AppState>) -> Result<(), DeleteError> {
    let smart_db = state.get_smart_db();
    if !does_role_exist(id, smart_db).await {
        return Err(DeleteError::NotFound);
    }
    if !can_delete_role(id, smart_db).await {
        return Err(DeleteError::Forbidden);
    }
    let view = DeleteRoleQueryView::new(id);
    smart_db
        .execute(view)
        .await
        .map_err(|_| DeleteError::DatabaseError)
}

#[utoipa::path(
    delete,
    path = "/{id}",
    responses(
        (status = 204, description = "Role deleted successfully"),
        (status = 403, description = "Role cannot be deleted"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = i32, Path, description = "Role database id")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Roles"
)]
#[delete("/{id}")]
pub async fn admin_delete_role(
    id: web::Path<u64>,
    state: web::Data<AppState>,
) -> Result<impl Responder, DeleteError> {
    delete_role(id.into_inner(), state).await?;
    Ok(HttpResponse::NoContent())
}
