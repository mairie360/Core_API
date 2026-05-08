use crate::endpoints::v1::groups::post::view::PostGroupView;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum PostGroupError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for PostGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            PostGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for PostGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            PostGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
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
) -> Result<(), PostGroupError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(PostGroupError::DatabaseError),
    };

    Ok(())
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
pub async fn post(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    view: web::Json<PostGroupView>,
) -> Result<impl Responder, PostGroupError> {
    create_group(user, state, view.into_inner()).await?;
    Ok(HttpResponse::Ok().body("Group created successfully"))
}
