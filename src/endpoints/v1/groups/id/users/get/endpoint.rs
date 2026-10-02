use crate::database::groups::does_group_exist::DoesGroupExistQuerView;
use crate::database::groups::get_group_members::GetGroupUsersQueryView;
use crate::endpoints::v1::groups::id::read_access::can_read_group;
use crate::endpoints::v1::groups::id::users::get::view::GetGroupUsersResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum GetUsersGroupError {
    DatabaseError,
    Forbidden,
    UnknowGroup,
}

impl std::fmt::Display for GetUsersGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
            Self::Forbidden => write!(f, "Insufficient permissions"),
            Self::UnknowGroup => {
                write!(f, "Unknow group")
            }
        }
    }
}

impl ResponseError for GetUsersGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::UnknowGroup => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

fn database_error(error: &impl std::fmt::Display) -> GetUsersGroupError {
    eprintln!("Get group members DB Error: {error}");
    GetUsersGroupError::DatabaseError
}

async fn trigger_get_group_members(
    state: web::Data<AppState>,
    user_id: u64,
    group_id: u64,
) -> Result<GetGroupUsersResultView, GetUsersGroupError> {
    let smart_db = state.get_smart_db();

    let exists: bool = smart_db
        .fetch_scalar(&DoesGroupExistQuerView::new(group_id))
        .await
        .map_err(|e| database_error(&e))?;
    if !exists {
        return Err(GetUsersGroupError::UnknowGroup);
    }
    if !can_read_group(smart_db, user_id, group_id)
        .await
        .map_err(|e| database_error(&e))?
    {
        return Err(GetUsersGroupError::Forbidden);
    }

    let result: Vec<i32> = smart_db
        .fetch_all(&GetGroupUsersQueryView::new(group_id))
        .await
        .map_err(|e| database_error(&e))?;

    Ok(result.into())
}

#[utoipa::path(
    get,
    path = "",
    summary = "List the members of a group",
    description = "Returns the ids of the group's members. Only ids are returned: pass them to \
                   `GET /api/v1/user/?ids=1,2,3` for their names and addresses.\n\n\
                   Restricted to the group's members (the owner is one, added when the group is \
                   created) and to whoever holds the `read` right on it: a global `read_all` \
                   (administrators, mayor) or an ACL. Anyone else gets `403`.",
    params(
        ("group_id" = u64, Path, description = "Group id.", example = 3)
    ),
    responses(
        (
            status = 200,
            description = "Ids of the group's members.",
            body = GetGroupUsersResultView,
            example = json!({ "users": [1, 2, 5] })
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
            description = "The caller is not a member of the group and holds no `read` right on it.",
            body = String,
            content_type = "text/plain",
            example = json!("Insufficient permissions")
        ),
        (
            status = 404,
            description = "No group has this id.",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow group")
        ),
        (
            status = 500,
            description = "Database failure while checking the rights or reading the members.",
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
#[get("/")]
pub async fn get_group_members(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    group_id: web::Path<u64>,
) -> Result<impl Responder, GetUsersGroupError> {
    let result = trigger_get_group_members(state, user.id, group_id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
