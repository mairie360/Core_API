use crate::database::groups::get_group::Group;
use crate::database::groups::update_group::UpdateGroupQueryView;
use crate::endpoints::v1::groups::id::patch::view::{PatchGroupView, MAX_GROUP_NAME_LENGTH};
use actix_web::http::StatusCode;
use actix_web::{patch, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PatchGroupError {
    BadRequest,
    UnknownGroup,
    DatabaseError,
}

impl std::fmt::Display for PatchGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchGroupError::BadRequest => write!(f, "Bad request."),
            PatchGroupError::UnknownGroup => write!(f, "Unknow group"),
            PatchGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for PatchGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            PatchGroupError::BadRequest => StatusCode::BAD_REQUEST,
            PatchGroupError::UnknownGroup => StatusCode::NOT_FOUND,
            PatchGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
        .map_err(|error| {
            eprintln!("{:?}", error);
            PatchGroupError::DatabaseError
        })?;

    groups
        .into_iter()
        .next()
        .ok_or(PatchGroupError::UnknownGroup)
}

#[utoipa::path(
    patch,
    path = "",
    params(
        ("group_id" = u64, Path, description = "ID du groupe")
    ),
    request_body = PatchGroupView,
    responses(
        (status = 200, description = "Group updated successfully", body = Group),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Unknow group"),
        (status = 500, description = "Internal server error")
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
    view: web::Json<PatchGroupView>,
) -> Result<impl Responder, PatchGroupError> {
    let group = trigger_patch_group(state, id.into_inner(), view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(group))
}
