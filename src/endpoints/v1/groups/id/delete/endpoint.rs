use crate::endpoints::db_error;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_group::DeleteGroupQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteGroupError {
    DatabaseError,
}

impl std::fmt::Display for DeleteGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for DeleteGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_delete_group(state: web::Data<AppState>, id: u64) -> Result<(), DeleteGroupError> {
    let db_view = DeleteGroupQueryView::new(id);
    state.get_smart_db().execute(db_view).await.map_err(|e| {
        db_error::log("delete group", &e);
        DeleteGroupError::DatabaseError
    })?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "",
    summary = "Supprimer un groupe",
    description = "Deletes a group and the memberships of its members. The user accounts \
                   themselves are untouched.\n\n\
                   Requires the `update` right on the group (owner, `update_all`, or an ACL): \
                   any other caller gets `403` and the group is kept. An unknown group answers \
                   `404` from the rights check.",
    params(
        ("group_id" = u64, Path, description = "Identifiant du groupe.", example = 3)
    ),
    responses(
        (
            status = 204,
            description = "Group deleted. Empty body.",
        ),
        (
            status = 401,
            description = "En-tête `Authorization` absent, JWT invalide ou expiré, ou session révoquée.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "The caller has no `update` right on the group (owner, `update_all` such as administrators, or an ACL), checked by `access_guard_middleware` before the handler runs.",
            body = String,
            content_type = "text/plain",
            example = json!("Insufficient permissions")
        ),
        (
            status = 404,
            description = "No group has this id (`Resource not found`, from the rights check).",
            body = String,
            content_type = "text/plain",
            example = json!("Resource not found")
        ),
        (
            status = 500,
            description = "Database error, logged by the server.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
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
