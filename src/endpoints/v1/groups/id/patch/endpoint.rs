use crate::database::groups::get_group::Group;
use crate::database::groups::update_group::UpdateGroupQueryView;
use crate::endpoints::v1::groups::id::patch::view::{PatchGroupView, MAX_GROUP_NAME_LENGTH};
use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchGroupError {
    BadRequest,
    UnknownGroup,
    Duplicate,
    DatabaseError,
}

impl std::fmt::Display for PatchGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => write!(f, "Bad request."),
            Self::UnknownGroup => write!(f, "Unknow group"),
            Self::Duplicate => write!(f, "A group with this name already exists."),
            Self::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for PatchGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::UnknownGroup => StatusCode::NOT_FOUND,
            Self::Duplicate => StatusCode::CONFLICT,
            Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_patch_group(
    state: web::Data<AppState>,
    group_id: u64,
    view: PatchGroupView,
) -> Result<Group, PatchGroupError> {
    let name = view.name().map(str::trim);
    if view.name().is_none() && view.description().is_none() {
        return Err(PatchGroupError::BadRequest);
    }
    if name.is_some_and(|name| name.is_empty() || name.chars().count() > MAX_GROUP_NAME_LENGTH) {
        return Err(PatchGroupError::BadRequest);
    }

    let groups: Vec<Group> = state
        .get_smart_db()
        .fetch_all(&UpdateGroupQueryView::new(
            group_id,
            name,
            view.description(),
        ))
        .await
        .map_err(|error| match error {
            // `groups.name` is UNIQUE.
            ApiLibError::Database(DbError::UniqueViolation(_)) => PatchGroupError::Duplicate,
            error => {
                tracing::error!("{error:?}");
                PatchGroupError::DatabaseError
            }
        })?;

    groups
        .into_iter()
        .next()
        .ok_or(PatchGroupError::UnknownGroup)
}

#[utoipa::path(
    patch,
    path = "",
    summary = "Update a group",
    description = "Updates the name or the description of a group and returns the group as \
                   saved. Partial update: an absent or `null` field stays \
                   unchanged.\n\n\
                   At least one of the two fields must be given: an empty body is refused with \
                   `400`. The name, once trimmed, can be neither empty nor \
                   longer than 255 characters.\n\n\
                   Requires the `update` right on the group (owner, `update_all`, or an ACL): \
                   any other caller gets `403` and the group is left untouched.",
    params(
        ("group_id" = u64, Path, description = "Id of the group.", example = 3)
    ),
    request_body(
        content = PatchGroupView,
        description = "Fields to update. Both are optional, but at least one must be present.",
        example = json!({ "description": "Building permits and prior declarations processing" })
    ),
    responses(
        (
            status = 200,
            description = "Group updated.",
            body = Group,
            example = json!({
                "id": 3,
                "owner_id": 2,
                "name": "Urban planning department",
                "description": "Building permits and prior declarations processing"
            })
        ),
        (
            status = 400,
            description = "Malformed JSON body, no field to update, or a field breaking its rules: `name` 1 to 64 characters once trimmed, no control character; `description` at most 1000 characters, no control character other than line breaks and tabs.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `name`: must not be empty")
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
            description = "No group has this id (`Resource not found`, from the rights check).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow group")
        ),
        (
            status = 409,
            description = "`name` is already used by another group (group names are unique).",
            body = String,
            content_type = "text/plain",
            example = json!("A group with this name already exists.")
        ),
        (
            status = 500,
            description = "Database error during the update.",
            body = String,
            content_type = "text/plain",
            example = json!("An error occurred while accessing the database.")
        )
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[patch("/")]
pub async fn patch_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    id: web::Path<u64>,
    view: ValidatedJson<PatchGroupView>,
) -> Result<impl Responder, PatchGroupError> {
    let group = trigger_patch_group(state, id.into_inner(), view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(group))
}
