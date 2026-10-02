use crate::database::groups::add_user_to_group::AddUserToGroupQueryView;
use crate::endpoints::v1::groups::id::users::post::view::PostUserGroupView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PostUserGroupError {
    GroupMismatch,
    UnknowUser,
}

impl std::fmt::Display for PostUserGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GroupMismatch => write!(
                f,
                "The body `group_id` does not match the group of the path."
            ),
            Self::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for PostUserGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::GroupMismatch => StatusCode::BAD_REQUEST,
            Self::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

/// Adds the user to `group_id`, the group of the path: the one `access_guard_middleware` checked
/// the `update` right on. The body cannot name another group (MAIR-390).
async fn trigger_add_user_to_group(
    state: web::Data<AppState>,
    group_id: u64,
    view: PostUserGroupView,
) -> Result<(), PostUserGroupError> {
    if view
        .body_group_id()
        .is_some_and(|body_group| body_group != group_id)
    {
        return Err(PostUserGroupError::GroupMismatch);
    }
    let db_view = AddUserToGroupQueryView::new(group_id, view.user_id());
    state.get_smart_db().execute(db_view).await.map_err(|e| {
        eprintln!("Add user to group DB Error: {e}");
        PostUserGroupError::UnknowUser
    })?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    summary = "Add a user to a group",
    description = "Adds a user to the group of the path. From then on, the group appears in that \
                   user's `GET /api/v1/groups/`, and the user inherits the group's access rights \
                   in every API.\n\n\
                   Requires the `update` right on the group (owner, `update_all`, or an ACL).\n\n\
                   The group is always the `group_id` of the **path**, the one the right is \
                   checked on. The body `group_id` is deprecated and optional: when sent, it must \
                   equal the path's, otherwise the answer is `400`.",
    params(
        ("group_id" = u64, Path, description = "Id of the group to add the user to.", example = 3)
    ),
    request_body(
        content = PostUserGroupView,
        description = "User to add.",
        example = json!({ "user_id": 42 })
    ),
    responses(
        (
            status = 200,
            description = "User added to the group.",
            body = String,
            content_type = "text/plain",
            example = json!("User added to group successfully")
        ),
        (
            status = 400,
            description = "Malformed JSON body, missing `user_id`, or body `group_id` different from the path's `group_id`.",
            body = String,
            content_type = "text/plain",
            example = json!("The body `group_id` does not match the group of the path.")
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
            description = "The caller has no `update` right on the group.",
            body = String,
            content_type = "text/plain",
            example = json!("Insufficient permissions")
        ),
        (
            status = 404,
            description = "The group does not exist (`Resource not found`, from the rights check), `user_id` matches no user, or the user is already a member of the group (`Unknow user.`).",
            body = String,
            content_type = "text/plain",
            example = json!("Unknow user.")
        ),
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[post("/")]
pub async fn add_user_to_group(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    path: web::Path<u64>,
    view: web::Json<PostUserGroupView>,
) -> Result<impl Responder, PostUserGroupError> {
    trigger_add_user_to_group(state, path.into_inner(), view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("User added to group successfully"))
}
