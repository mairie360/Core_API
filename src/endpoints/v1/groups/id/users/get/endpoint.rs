use crate::endpoints::v1::groups::id::users::get::view::GetGroupUsersResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum GetUsersGroupError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for GetUsersGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUsersGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            GetUsersGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for GetUsersGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUsersGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetUsersGroupError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_group_users(
    state: web::Data<AppState>,
    group_id: i32,
) -> Result<GetGroupUsersResultView, GetUsersGroupError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetUsersGroupError::DatabaseError),
    };

    Ok(GetGroupUsersResultView::new(vec![]))
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, body = GetGroupUsersResultView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Groups",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get(
    _: AuthenticatedUser,
    state: web::Data<AppState>,
    group_id: web::Path<i32>,
) -> Result<impl Responder, GetUsersGroupError> {
    let result = get_group_users(state, group_id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
