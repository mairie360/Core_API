use crate::database::groups::get_user_groups::{get_user_groups, GetUserGroupsQuerView};
use crate::endpoints::v1::groups::get::view::GetGroupsResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum GetGroupsError {
    BadRequest,
    DatabaseError,
}

impl std::fmt::Display for GetGroupsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetGroupsError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            GetGroupsError::BadRequest => {
                write!(f, "Bad request.")
            }
        }
    }
}

impl ResponseError for GetGroupsError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetGroupsError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetGroupsError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_groups(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<GetGroupsResultView, GetGroupsError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetGroupsError::DatabaseError),
    };

    let groups = get_user_groups(GetUserGroupsQuerView::new(user.id), pool)
        .await
        .map_err(|_| GetGroupsError::BadRequest)?;

    Ok(groups.into())
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, body = GetGroupsResultView),
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
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, GetGroupsError> {
    let result = get_groups(user, state).await?;
    Ok(HttpResponse::Ok().json(result))
}
