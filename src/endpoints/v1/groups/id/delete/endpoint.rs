use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_group::{delete_group_query, DeleteGroupQueryView};

#[derive(Debug, Clone, PartialEq)]
enum DeleteGroupError {
    BadRequest,
}

impl std::fmt::Display for DeleteGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for DeleteGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            DeleteGroupError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_delete_group(state: web::Data<AppState>, id: u64) -> Result<(), DeleteGroupError> {
    let db_view = DeleteGroupQueryView::new(id);
    delete_group_query(db_view, state.get_smart_db())
        .await
        .map_err(|_| DeleteGroupError::BadRequest)?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    params(
        ("group_id" = u64, Path, description = "ID du groupe")
    ),
    responses(
        (status = 204, description = "Group deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn delete_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
) -> Result<impl Responder, DeleteGroupError> {
    trigger_delete_group(state, id.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}
