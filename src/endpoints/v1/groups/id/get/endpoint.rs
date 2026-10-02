use crate::database::groups::does_group_exist::DoesGroupExistQuerView;
use crate::database::groups::get_group::GetGroupQuerView;
use crate::endpoints::v1::groups::id::get::view::GetGroupResultView;
use crate::endpoints::v1::groups::id::read_access::can_read_group;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum GetGroupError {
    DatabaseError,
    Forbidden,
    UnknowGroup,
}

impl std::fmt::Display for GetGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError => write!(f, "An error occurred while accessing the database."),
            Self::Forbidden => write!(f, "Insufficient permissions"),
            Self::UnknowGroup => {
                write!(f, "Unknow group.")
            }
        }
    }
}

impl ResponseError for GetGroupError {
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

fn database_error(error: &impl std::fmt::Display) -> GetGroupError {
    tracing::error!("Get group DB Error: {error}");
    GetGroupError::DatabaseError
}

async fn trigger_get_group(
    state: web::Data<AppState>,
    user_id: u64,
    id: u64,
) -> Result<GetGroupResultView, GetGroupError> {
    let smart_db = state.get_smart_db();

    let exists: bool = smart_db
        .fetch_scalar(&DoesGroupExistQuerView::new(id))
        .await
        .map_err(|e| database_error(&e))?;
    if !exists {
        return Err(GetGroupError::UnknowGroup);
    }
    if !can_read_group(smart_db, user_id, id)
        .await
        .map_err(|e| database_error(&e))?
    {
        return Err(GetGroupError::Forbidden);
    }

    let result = smart_db
        .fetch_one(&GetGroupQuerView::new(id))
        .await
        .map_err(|e| database_error(&e))?;

    Ok(GetGroupResultView::new(result))
}

#[utoipa::path(
    get,
    path = "",
    summary = "Read a group",
    description = "Returns the name, description and owner of a group.\n\n\
                   Restricted to the group's members (the owner is one) and to whoever holds the \
                   `read` right on it: a global `read_all` (administrators, mayor) or an ACL. \
                   Anyone else gets `403`.\n\n\
                   For its member list, see `GET /api/v1/groups/{group_id}/users/`.",
    params(
        ("group_id" = u64, Path, description = "Group id.", example = 3)
    ),
    responses(
        (
            status = 200,
            description = "The requested group.",
            body = GetGroupResultView,
            example = json!({
                "group": { "id": 3, "owner_id": 2, "name": "Service urbanisme", "description": "Instruction des permis de construire" }
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
            example = json!("Unknow group.")
        ),
        (
            status = 500,
            description = "Database failure while checking the rights or reading the group.",
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
pub async fn get_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
) -> Result<impl Responder, GetGroupError> {
    let result = trigger_get_group(state, user.id, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
