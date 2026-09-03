use crate::database::groups::create_group::CreateGroupQueryView;
use crate::endpoints::v1::groups::post::view::{PostGroupResultView, PostGroupView};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::security::AuthenticatedUser;
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum PostGroupError {
    BadRequest,
}

impl std::fmt::Display for PostGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for PostGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            PostGroupError::BadRequest => StatusCode::BAD_REQUEST,
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
    let id: i32 = state
        .get_smart_db()
        .fetch_scalar(&db_view)
        .await
        .map_err(|_| PostGroupError::BadRequest)?;

    Ok(PostGroupResultView::new(id as u64))
}

#[utoipa::path(
    post,
    path = "",
    request_body = PostGroupView,
    responses(
        (status = 200, description = "Group created successfully"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
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
    view: web::Json<PostGroupView>,
) -> Result<impl Responder, PostGroupError> {
    let result = create_group(user, state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
