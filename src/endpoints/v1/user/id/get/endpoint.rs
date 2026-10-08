use crate::database::groups::get_user_groups::GetUserGroupsQuerView;
use crate::database::roles::get_roles_by_id::GetRolesByIdQueryView;
use crate::database::users::get_roles::GetUserRolesQueryView;
use crate::database::users::get_user_by_id::GetUserByIdQueryView;
use crate::endpoints::v1::user::directory_access::may_read_directory;
use crate::endpoints::v1::user::id::get::view::GetUserResponseView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::database::query_views::IsAdminQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    DatabaseError,
    Forbidden,
    UnknownUser,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            Self::Forbidden => write!(f, "Forbidden: the directory is closed to guests."),
            Self::UnknownUser => {
                write!(f, "User not found.")
            }
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::UnknownUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_get_user(
    state: web::Data<AppState>,
    caller_id: u64,
    id: u64,
) -> Result<GetUserResponseView, GetUserError> {
    let smart_db = state.get_smart_db();
    if caller_id != id
        && !may_read_directory(smart_db, caller_id).await.map_err(|e| {
            tracing::error!("Get user DB Error: {e}");
            GetUserError::DatabaseError
        })?
    {
        return Err(GetUserError::Forbidden);
    }

    let view = GetUserByIdQueryView::new(id);
    let result: crate::database::users::get_user_by_id::GetUserByIdQueryResultView =
        match smart_db.fetch_one(&view).await {
            Ok(result) => result,
            Err(ApiLibError::Database(DbError::NotFound)) => return Err(GetUserError::UnknownUser),
            Err(e) => {
                tracing::error!("Get user DB Error: {e}");
                return Err(GetUserError::DatabaseError);
            }
        };
    // An archived account is only visible to itself and to the administrators (MAIR-390,
    // MAIR-288): for everyone else it answers as if it did not exist. The phone number is visible
    // to every agent (decision of the mairie, MAIR-288: professional use).
    let full_access = caller_id == id
        || smart_db
            .fetch_scalar::<bool, _>(&IsAdminQueryView::new(caller_id))
            .await
            .map_err(|e| {
                tracing::error!("Get user DB Error: {e}");
                GetUserError::DatabaseError
            })?;
    if !full_access && result.is_archived() {
        return Err(GetUserError::UnknownUser);
    }
    let view = GetUserGroupsQuerView::new(id);
    let groups = smart_db.fetch_all(&view).await.map_err(|e| {
        tracing::error!("Get user DB Error: {e}");
        GetUserError::DatabaseError
    })?;
    let role = GetUserRolesQueryView::new(id);
    let role_id: Vec<i32> = smart_db.fetch_all(&role).await.map_err(|e| {
        tracing::error!("Get user DB Error: {e}");
        GetUserError::DatabaseError
    })?;
    let view = GetRolesByIdQueryView::new(role_id);
    let role: Vec<crate::database::roles::get_roles_by_id::Role> =
        smart_db.fetch_all(&view).await.map_err(|e| {
            tracing::error!("Get user DB Error: {e}");
            GetUserError::DatabaseError
        })?;

    Ok(GetUserResponseView::new(
        result.first_name(),
        result.last_name(),
        result.email(),
        result.phone_e164(),
        result.phone_country(),
        result.status(),
        result.is_archived(),
        role.iter().map(|r| r.name().to_string()).collect(),
        groups,
    ))
}

#[utoipa::path(
    get,
    path = "/",
    summary = "Read a user's record",
    description = "Returns a user's record: names, e-mail, phone, status, archive flag, roles and \
                   groups. Open to every authenticated user, with restrictions on other users' \
                   records:\n\n\
                   - the user themself and administrators also read archived accounts \
                   (`is_archived` is then `true`);\n\
                   - for anyone else an archived account answers `404` as if it did not exist, so \
                   `is_archived` is always `false` for them.\n\n\
                   The phone number is visible to every agent (MAIR-288). With \
                   `DIRECTORY_GUEST_ACCESS=none`, an agent who only holds the Guest role gets `403` \
                   on any record but their own.\n\n\
                   `role` is deprecated: read `roles`, which lists every role.",
    params(
        ("id" = u64, Path, description = "Id of the user to read.", example = 42)
    ),
    responses(
        (
            status = 200,
            description = "The user's record.",
            body = GetUserResponseView,
            example = json!({
                "first_name": "Jean",
                "last_name": "Dupont",
                "email": "jean.dupont@mairie360.fr",
                "phone": "+33612345678",
                "phone_country": "FR",
                "status": "active",
                "is_archived": false,
                "role": "agent",
                "roles": ["agent"],
                "groups": [
                    { "id": 3, "owner_id": 2, "name": "Service urbanisme", "description": "Instruction des permis de construire" }
                ]
            })
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Unauthorized")
        ),
        (
            status = 403,
            description = "`DIRECTORY_GUEST_ACCESS=none` and the caller only holds the Guest role (their own record stays readable).",
            body = String,
            content_type = "text/plain",
            example = json!("Forbidden: the directory is closed to guests.")
        ),
        (
            status = 404,
            description = "No user has this id, or the account is archived and the caller is neither that user nor an administrator.",
            body = String,
            content_type = "text/plain",
            example = json!("User not found.")
        ),
        (
            status = 500,
            description = "Database failure while reading the user.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_user(
    caller: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<String>,
) -> Result<impl Responder, GetUserError> {
    let user = trigger_get_user(state, caller.id, id.parse::<u64>().unwrap_or(0)).await?;
    Ok(HttpResponse::Ok().json(user))
}
