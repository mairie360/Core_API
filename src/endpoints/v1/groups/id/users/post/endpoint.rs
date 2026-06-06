use crate::database::groups::add_user_to_group::{
    add_user_to_group_query, AddUserToGroupQueryView,
};
use crate::endpoints::v1::groups::id::users::post::view::PostUserGroupView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum PostUserGroupError {
    // BadRequest,
    DatabaseError,
    UnknowUser,
}

impl std::fmt::Display for PostUserGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostUserGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
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
            PostUserGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            PostUserGroupError::UnknowUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn add_user_to_group(
    state: web::Data<AppState>,
    view: PostUserGroupView,
) -> Result<(), PostUserGroupError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(PostUserGroupError::DatabaseError),
    };

    let db_view = AddUserToGroupQueryView::new(view.user_id(), view.group_id());
    add_user_to_group_query(db_view, pool)
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
pub async fn post(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    view: web::Json<PostUserGroupView>,
) -> Result<impl Responder, PostUserGroupError> {
    add_user_to_group(state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("User added to group successfully"))
}
