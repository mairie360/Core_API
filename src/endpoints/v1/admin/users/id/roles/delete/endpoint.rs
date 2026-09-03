use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

use crate::database::users::remove_role::{remove_role_query, RemoveRolesQueryView};

#[derive(Debug, Clone, PartialEq)]
enum RemoveUserRoleError {
    NotFound,
}

impl std::fmt::Display for RemoveUserRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveUserRoleError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for RemoveUserRoleError {
    fn status_code(&self) -> StatusCode {
        match self {
            RemoveUserRoleError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user(
    state: web::Data<AppState>,
    user_id: u64,
    role_id: u64,
) -> Result<(), RemoveUserRoleError> {
    let view = RemoveRolesQueryView::new(role_id, user_id);
    remove_role_query(view, state.get_smart_db())
        .await
        .map_err(|_| RemoveUserRoleError::NotFound)?;

    Ok(())
}

#[utoipa::path(
    delete,
    params(
        ("roleId" = i32, Path, description = "ID du rôle"),
        ("userId" = i32, Path, description = "ID de l'utilisateur")
    ),
    path = "/{roleId}",
    responses(
        (status = 204, description = "Role deleted successfully"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/{roleId}")]
pub async fn admin_delete_user_role(
    state: web::Data<AppState>,
    params: web::Path<(u64, u64)>,
) -> Result<impl Responder, RemoveUserRoleError> {
    let (user_id, role_id) = params.into_inner();
    delete_user(state, user_id, role_id).await?;
    Ok(HttpResponse::NoContent())
}
