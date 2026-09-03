use crate::database::groups::add_user_to_group::AddUserToGroupQueryView;
use crate::endpoints::v1::groups::id::users::post::view::PostUserGroupView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostUserGroupError {
    // BadRequest,
    UnknowUser,
}

impl std::fmt::Display for PostUserGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // PostUserGroupError::BadRequest => {
            //     write!(f, "Bad request.")
            // }
            PostUserGroupError::UnknowUser => {
                write!(f, "Unknow user.")
            }
        }
    }
}

impl ResponseError for PostUserGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            // PostUserGroupError::BadRequest => StatusCode::BAD_REQUEST,
            PostUserGroupError::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn trigger_add_user_to_group(
    state: web::Data<AppState>,
    view: PostUserGroupView,
) -> Result<(), PostUserGroupError> {
    let db_view = AddUserToGroupQueryView::new(view.user_id(), view.group_id());
    state
        .get_smart_db()
        .execute(db_view)
        .await
        .map_err(|_| PostUserGroupError::UnknowUser)?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "",
    params(
        ("group_id" = u64, Path, description = "ID du groupe")
    ),
    request_body = PostUserGroupView,
    responses(
        (status = 200, description = "User added to group successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Unknow user."),
        (status = 500, description = "Internal server error")
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
    view: web::Json<PostUserGroupView>,
) -> Result<impl Responder, PostUserGroupError> {
    trigger_add_user_to_group(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("User added to group successfully"))
}
