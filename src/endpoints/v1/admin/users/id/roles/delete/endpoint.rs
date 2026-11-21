use crate::endpoints::admin_guard::AdminUser;
use actix_web::http::StatusCode;
use actix_web::{delete, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

use crate::database::users::remove_role::RemoveRolesQueryView;
use crate::endpoints::db_error;
use crate::endpoints::last_admin::{self, LAST_ADMIN_MESSAGE};

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoveUserRoleError {
    LastAdmin,
    DatabaseError,
}

impl std::fmt::Display for RemoveUserRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LastAdmin => write!(f, "{LAST_ADMIN_MESSAGE}"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for RemoveUserRoleError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::LastAdmin => StatusCode::CONFLICT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn database_error(e: &ApiLibError) -> RemoveUserRoleError {
    db_error::log("revoke role", e);
    RemoveUserRoleError::DatabaseError
}

/// Revokes the role in one transaction guarded against removing the `Admin` role of the last
/// active administrator (MAIR-678): the `Admin` role is locked first, the role revoked, and the
/// transaction rolled back with [`RemoveUserRoleError::LastAdmin`] when no active administrator
/// is left.
async fn revoke_in_core(
    smart_db: &SmartDatabase,
    user_id: u64,
    role_id: u64,
) -> Result<(), RemoveUserRoleError> {
    let mut tx = smart_db.begin().await.map_err(|e| database_error(&e))?;
    last_admin::lock(&mut tx)
        .await
        .map_err(|e| database_error(&e))?;
    tx.execute(&RemoveRolesQueryView::new(role_id, user_id))
        .await
        .map_err(|e| database_error(&e))?;
    if !last_admin::any_left(&mut tx)
        .await
        .map_err(|e| database_error(&e))?
    {
        return Err(RemoveUserRoleError::LastAdmin);
    }
    tx.commit().await.map_err(|e| database_error(&e))
}

async fn remove_role_from_user(
    state: web::Data<AppState>,
    user_id: u64,
    role_id: u64,
) -> Result<(), RemoveUserRoleError> {
    revoke_in_core(state.get_smart_db(), user_id, role_id).await
}

#[utoipa::path(
    delete,
    summary = "Revoke a role from a user",
    description = "Detaches a role from a user account. The role itself and the account are \
                   kept; only the assignment disappears. Reserved to administrators.\n\n\
                   Unlike the grant, both ids are read from the path. Revoking a role the user \
                   does not hold, or naming an unknown user or role, matches no row and still \
                   answers `204`; `404` is reserved to a failed write.\n\n\
                   **Last administrator (MAIR-678).** An instance keeps at least one active \
                   administrator (role `Admin`, account not archived): revoking `Admin` from the \
                   only one answers `409` and changes nothing. The check \
                   and the write run in one transaction locking the `Admin` role, so two \
                   concurrent revocations from the two last administrators cannot both succeed.",
    params(
        ("userId" = u64, Path, description = "User id.", example = 42),
        ("roleId" = u64, Path, description = "Id of the role to revoke.", example = 2)
    ),
    path = "/{roleId}",
    responses(
        (
            status = 204,
            description = "Role revoked, or nothing to revoke. Empty body.",
        ),
        (
            status = 400,
            description = "`userId` or `roleId` is not an integer.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid path: ids are integers.")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
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
            status = 409,
            description = "`roleId` is `Admin` and the user is the last active administrator (MAIR-678): nothing changes.",
            body = String,
            content_type = "text/plain",
            example = json!("This user is the last active administrator: give the Admin role to another active user first.")
        ),
        (
            status = 500,
            description = "The role or the account could not be read, or the write failed. The cause is logged by the server.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        ),
    ),
    security(
        ("jwt" = [])
    ),
    tag = "Admin - Users"
)]
#[delete("/{roleId}")]
pub async fn admin_delete_user_role(
    _: AdminUser,
    state: web::Data<AppState>,
    params: web::Path<(u64, u64)>,
) -> Result<impl Responder, RemoveUserRoleError> {
    let (user_id, role_id) = params.into_inner();
    remove_role_from_user(state, user_id, role_id).await?;
    Ok(HttpResponse::NoContent())
}
