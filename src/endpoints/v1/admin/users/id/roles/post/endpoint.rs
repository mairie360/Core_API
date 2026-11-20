use crate::database::users::add_role::AddRolesQueryView;
use crate::endpoints::admin_guard::AdminUser;
use crate::endpoints::db_error::{self, DbFailure};
use crate::endpoints::last_admin::{self, LAST_ADMIN_MESSAGE};
use crate::endpoints::v1::admin::users::id::roles::post::view::AddRoleToUserView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum AddRoleToUserError {
    UserMismatch,
    NotFound,
    AlreadyGranted,
    LastAdmin,
    DatabaseError,
}

impl std::fmt::Display for AddRoleToUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserMismatch => {
                write!(f, "The body `user_id` does not match the user of the path.")
            }
            Self::NotFound => {
                write!(f, "User or role not found.")
            }
            Self::AlreadyGranted => write!(f, "The user already holds this role."),
            Self::LastAdmin => write!(f, "{LAST_ADMIN_MESSAGE}"),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for AddRoleToUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UserMismatch => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AlreadyGranted | Self::LastAdmin => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type("text/plain; charset=utf-8")
            .body(self.to_string())
    }
}

fn grant_failure(e: &ApiLibError) -> AddRoleToUserError {
    match db_error::log("grant role", e) {
        // `user_id` or `role_id` matches nothing (foreign key).
        DbFailure::NotFound => AddRoleToUserError::NotFound,
        DbFailure::Conflict => AddRoleToUserError::AlreadyGranted,
        _ => AddRoleToUserError::DatabaseError,
    }
}

/// Grants the role in one transaction guarded against demoting the last active administrator
/// (MAIR-678): the `Admin` role is locked first, the role written, and the transaction rolled
/// back with [`AddRoleToUserError::LastAdmin`] when no active administrator is left.
async fn grant_in_core(
    smart_db: &SmartDatabase,
    user_id: u64,
    view: &AddRoleToUserView,
) -> Result<(), AddRoleToUserError> {
    let mut tx = smart_db.begin().await.map_err(|e| grant_failure(&e))?;
    last_admin::lock(&mut tx)
        .await
        .map_err(|e| grant_failure(&e))?;
    // Replaces the user's role (one role per user, MAIR-564); no row when it was already held.
    let granted: Vec<i32> = tx
        .fetch_all(&AddRolesQueryView::new(view.role_id(), user_id))
        .await
        .map_err(|e| grant_failure(&e))?;
    if granted.is_empty() {
        return Err(AddRoleToUserError::AlreadyGranted);
    }
    if !last_admin::any_left(&mut tx)
        .await
        .map_err(|e| grant_failure(&e))?
    {
        return Err(AddRoleToUserError::LastAdmin);
    }
    tx.commit().await.map_err(|e| grant_failure(&e))
}

async fn add_role_to_user(
    state: web::Data<AppState>,
    user_id: u64,
    view: AddRoleToUserView,
) -> Result<(), AddRoleToUserError> {
    // The user is the one of the path (MAIR-390).
    if view
        .body_user_id()
        .is_some_and(|body_user| body_user != user_id)
    {
        return Err(AddRoleToUserError::UserMismatch);
    }
    grant_in_core(state.get_smart_db(), user_id, &view).await
}

#[utoipa::path(
    post,
    path = "/",
    summary = "Set the role of a user",
    description = "Gives a role to a user account, **replacing the role it held**: a user holds \
                   one role (MAIR-564). An account registered without a role holds `Guest` until \
                   then. Reserved to administrators. The new role reaches the user's JWT at its \
                   next refresh or login.\n\n\
                   The target user is always the `userId` of the **path**. The body `user_id` is \
                   deprecated and optional: when sent, it must equal the path's, otherwise the \
                   answer is `400`.\n\n\
                   The response has an empty body. Every Core write failure is reported as \
                   `404`.\n\n\
                   **Last administrator (MAIR-678).** An instance keeps at least one active \
                   administrator (role `Admin`, account not archived): giving another role to the \
                   only one answers `409` and changes nothing. The check \
                   and the write run in one transaction locking the `Admin` role, so two \
                   concurrent demotions of the two last administrators cannot both succeed.",
    request_body(
        content = AddRoleToUserView,
        description = "Role to grant.",
        example = json!({ "role_id": 2 })
    ),
    params(
        ("userId" = u64, Path, description = "Id of the user receiving the role.", example = 42)
    ),
    responses(
        (
            status = 200,
            description = "Role set, the former one replaced. Empty body.",
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing `role_id`, or body `user_id` different from the path's `userId`.",
            body = String,
            content_type = "text/plain",
            example = json!("The body `user_id` does not match the user of the path.")
        ),
        (
            status = 401,
            description = "`Authorization` header missing, JWT invalid or expired, or session revoked.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized")
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
            description = "`user_id` or `role_id` matches nothing.",
            body = String,
            content_type = "text/plain",
            example = json!("User or role not found.")
        ),
        (
            status = 409,
            description = "The user already holds the role, or the user is the last active administrator and the new role is not `Admin` (MAIR-678): nothing changes.",
            body = String,
            content_type = "text/plain",
            examples(
                ("Already held" = (value = json!("The user already holds this role."))),
                ("Last administrator" = (value = json!("This user is the last active administrator: give the Admin role to another active user first.")))
            )
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
#[post("/")]
pub async fn admin_add_role_to_user(
    _: AdminUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<AddRoleToUserView>,
) -> Result<impl Responder, AddRoleToUserError> {
    add_role_to_user(state, path.into_inner(), view.into_inner()).await?;
    Ok(HttpResponse::Ok())
}
