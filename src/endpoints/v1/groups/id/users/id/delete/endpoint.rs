use crate::endpoints::db_error;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

use crate::database::groups::delete_user_from_group::DeleteUserFromGroupQueryView;

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserFromGroupError {
    DatabaseError,
    UnknowUser,
}

impl std::fmt::Display for DeleteUserFromGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for DeleteUserFromGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn delete_user_from_group(
    state: web::Data<AppState>,
    group_id: u64,
    user_id: u64,
) -> Result<(), DeleteUserFromGroupError> {
    let removed: i64 = state
        .get_smart_db()
        .fetch_scalar(&DeleteUserFromGroupQueryView::new(group_id, user_id))
        .await
        .map_err(|e| {
            db_error::log("remove user from group", &e);
            DeleteUserFromGroupError::DatabaseError
        })?;
    if removed == 0 {
        return Err(DeleteUserFromGroupError::UnknowUser);
    }
    Ok(())
}

#[utoipa::path(
    delete,
    path = "/",
    summary = "Remove a user from a group",
    description = "Removes a user from a group. The user account and the group are \
                   kept; only the membership goes away.\n\n\
                   Unlike the deletion of a group, this call is not idempotent: \
                   removing a user who is not a member answers `404`.\n\n\
                   Requires the `update` right on the group (owner, `update_all`, or an ACL): \
                   any other caller gets `403` and the membership is kept.",
    params(
        ("group_id" = u64, Path, description = "Id of the group.", example = 3),
        ("user_id" = u64, Path, description = "Id of the user to remove.", example = 42)
    ),
    responses(
        (
            status = 204,
            description = "User removed from the group. Empty body.",
        ),
        (
            status = 400,
            description = "`group_id` or `user_id` in the path is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid ID format in URL")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
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
            description = "The user is not a member of this group, or one of the two ids does not exist.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow user.")
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
pub async fn remove_user_from_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<(u64, u64)>,
) -> Result<impl Responder, DeleteUserFromGroupError> {
    let (group_id, user_id) = path.into_inner();
    delete_user_from_group(state, group_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
