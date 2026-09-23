use actix_web::{delete, error::ResponseError, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::state::AppState;

use crate::database::users::delete_user::{DeleteUserQueryView, IsUserActiveQueryView};

/// SQLSTATE raised by `fn_check_can_delete_user` when the user still owns resources.
const RESTRICT_VIOLATION: &str = "23001";

#[derive(Debug, Clone, PartialEq)]
enum DeleteUserError {
    UnknownUser,
    OwnsResources,
    DatabaseError,
}

impl std::fmt::Display for DeleteUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownUser => write!(f, "Unknown user"),
            Self::OwnsResources => write!(
                f,
                "The user still owns groups, events or projects: transfer them first"
            ),
            Self::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

impl ResponseError for DeleteUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UnknownUser => StatusCode::NOT_FOUND,
            Self::OwnsResources => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn is_restrict_violation(error: &ApiLibError) -> bool {
    matches!(
        error,
        ApiLibError::Database(DbError::Sqlx(sqlx::Error::Database(db_error)))
            if db_error.code().as_deref() == Some(RESTRICT_VIOLATION)
    )
}

async fn delete_user(state: web::Data<AppState>, user_id: u64) -> Result<(), DeleteUserError> {
    let smart_db = state.get_smart_db();
    let active: bool = smart_db
        .fetch_scalar(&IsUserActiveQueryView::new(user_id))
        .await
        .map_err(|e| {
            eprintln!("Error: {e}");
            DeleteUserError::DatabaseError
        })?;
    if !active {
        return Err(DeleteUserError::UnknownUser);
    }

    smart_db
        .execute(DeleteUserQueryView::new(user_id))
        .await
        .map_err(|e| {
            if is_restrict_violation(&e) {
                DeleteUserError::OwnsResources
            } else {
                eprintln!("Error: {e}");
                DeleteUserError::DatabaseError
            }
        })
}

#[utoipa::path(
    delete,
    path = "",
    summary = "Delete a user (administration)",
    description = "Archives a user account: it is flagged `archived` and leaves the active users \
                   (directory, admin listing), but is never hard-deleted. Administrators only.\n\n\
                   Refused with `409` while the user still owns groups, events or projects: \
                   transfer them first. An unknown or already archived user answers `404`.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42)
    ),
    responses(
        (
            status = 204,
            description = "Account archived. Empty body.",
        ),
        (
            status = 400,
            description = "`userId` in the path is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("can not parse \"abc\" to a u64")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 403,
            description = "The user is authenticated but is not an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: User is not an admin.")
        ),
        (
            status = 404,
            description = "No active user matches `userId` (unknown or already archived).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknown user")
        ),
        (
            status = 409,
            description = "The user still owns groups, events or projects.",
            body = String,
            content_type = "text/plain",
            example = json!("The user still owns groups, events or projects: transfer them first")
        ),
        (
            status = 500,
            description = "Database error while checking or archiving the user.",
            body = String,
            content_type = "text/plain",
            example = json!("Database error occurred")
        ),
    ),
    tag = "Admin - Users",
    security(
        ("jwt" = [])
    )
)]
#[delete("/")]
pub async fn admin_delete_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, DeleteUserError> {
    delete_user(state, path.into_inner()).await?;

    Ok(HttpResponse::NoContent())
}
