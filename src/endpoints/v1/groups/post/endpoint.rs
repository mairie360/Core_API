use crate::database::groups::create_group::CreateGroupQueryView;
use crate::database::ids::id_from_sql;
use crate::endpoints::v1::groups::post::view::{PostGroupResultView, PostGroupView};
use crate::endpoints::validation::ValidatedJson;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostGroupError {
    BadRequest,
    Duplicate,
}

impl std::fmt::Display for PostGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest => {
                write!(f, "Bad request.")
            }
            Self::Duplicate => {
                write!(f, "A group with this name already exists.")
            }
        }
    }
}

impl ResponseError for PostGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Duplicate => StatusCode::CONFLICT,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn create_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: PostGroupView,
) -> Result<PostGroupResultView, PostGroupError> {
    let db_view = CreateGroupQueryView::new(user.id, view.name(), view.description());
    let id: i32 =
        state
            .get_smart_db()
            .fetch_scalar(&db_view)
            .await
            .map_err(|error| match error {
                // `groups.name` is UNIQUE.
                ApiLibError::Database(DbError::UniqueViolation(_)) => PostGroupError::Duplicate,
                _ => PostGroupError::BadRequest,
            })?;

    Ok(PostGroupResultView::new(id_from_sql(id)))
}

#[utoipa::path(
    post,
    path = "",
    summary = "Create a group",
    description = "Creates a group whose owner (`owner_id`) is the user of the JWT. \
                   They are not a member of it for all that: add them \
                   explicitly through `POST /api/v1/groups/{group_id}/users/` for it to \
                   show up in `GET /api/v1/groups/`.\n\n\
                   The response only holds the assigned id; read the full group again \
                   with `GET /api/v1/groups/{group_id}/`.",
    request_body(
        content = PostGroupView,
        description = "Name and description of the group. Both fields are required; pass an empty string for a missing description.",
        example = json!({
            "name": "Urban planning department",
            "description": "Building permit processing"
        })
    ),
    responses(
        (
            status = 200,
            description = "Group created. The body holds the assigned id.",
            body = PostGroupResultView,
            example = json!({ "id": 3 })
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing field, a field breaking its rules (`name` 1 to 64 characters once trimmed, no control character; `description` at most 1000 characters, no control character other than line breaks and tabs), or failed insert. This endpoint answers `400` where others would answer `500`.",
            body = String,
            content_type = "text/plain",
            example = json!("Invalid `name`: must be at most 64 characters")
        ),
        (
            status = 401,
            description = "Missing `Authorization` header, invalid or expired JWT, or revoked session.",
            body = String,
            content_type = "text/plain",
            example = json!("Jeton expiré")
        ),
        (
            status = 409,
            description = "`name` is already used by another group (group names are unique).",
            body = String,
            content_type = "text/plain",
            example = json!("A group with this name already exists.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn post_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: ValidatedJson<PostGroupView>,
) -> Result<impl Responder, PostGroupError> {
    let result = create_group(user, state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
