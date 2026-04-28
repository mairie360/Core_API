use crate::database::roles::does_role_exist::{does_role_exist_query, DoesRoleExistQueryView};
use crate::database::roles::patch_role::{patch_role_query, PatchRoleQueryView};
use crate::endpoints::v1::admin::roles::patch::view::PatchView;

use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq)]
enum PatchError {
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            PatchError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for PatchError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            PatchError::NotFound => StatusCode::NOT_FOUND,
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

async fn patch_role(
    id: u64,
    payload: PatchView,
    state: web::Data<AppState>,
) -> Result<(), PatchError> {
    if !does_role_exist(id, state.db_pool.clone()).await {
        return Err(PatchError::NotFound);
    }
    let view = PatchRoleQueryView::new(
        id,
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );
    patch_role_query(view, state.db_pool.clone())
        .await
        .map_err(|_| PatchError::DatabaseError)?;
    Ok(())
}

#[utoipa::path(
    patch,
    path = "/{id}",
    request_body = PatchView,
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Resource not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = i32, Path, description = "Role database id") // <--- AJOUTE CECI
    ),
    security(
        ("jwt" = [])
    )
)]
#[patch("/{id}")]
pub async fn patch(
    id: web::Path<u64>,
    payload: web::Json<PatchView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PatchError> {
    patch_role(id.into_inner(), payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
