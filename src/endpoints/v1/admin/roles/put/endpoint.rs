use crate::database::roles::change_role::{change_role_query, ChangeRoleQueryView};
use crate::database::roles::does_role_exist::{does_role_exist_query, DoesRoleExistQueryView};
use crate::endpoints::v1::admin::roles::view::RoleWriteView;

use actix_web::http::StatusCode;
use actix_web::{put, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq)]
enum PutError {
    NotFound,
    DatabaseError,
}

impl std::fmt::Display for PutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PutError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            PutError::NotFound => {
                write!(f, "The requested resource was not found.")
            }
        }
    }
}

impl ResponseError for PutError {
    fn status_code(&self) -> StatusCode {
        match self {
            PutError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            PutError::NotFound => StatusCode::NOT_FOUND,
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

async fn put_role(
    id: u64,
    payload: RoleWriteView,
    state: web::Data<AppState>,
) -> Result<(), PutError> {
    if !does_role_exist(id, state.db_pool.clone().unwrap()).await {
        return Err(PutError::NotFound);
    }
    let view = ChangeRoleQueryView::new(
        id,
        payload.name(),
        payload.description(),
        payload.can_be_deleted(),
    );
    change_role_query(view, state.db_pool.clone().unwrap())
        .await
        .map_err(|_| PutError::DatabaseError)?;
    Ok(())
}

#[utoipa::path(
    put,
    path = "/{id}",
    request_body = RoleWriteView,
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 400, description = "Bad request"),
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
#[put("/{id}")]
pub async fn put(
    id: web::Path<u64>,
    payload: web::Json<RoleWriteView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, PutError> {
    put_role(id.into_inner(), payload.into_inner(), state).await?;
    Ok(HttpResponse::Ok())
}
