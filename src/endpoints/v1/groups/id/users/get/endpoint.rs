use crate::database::groups::does_group_exist::{does_group_exist_query, DoesGroupExistQuerView};
use crate::database::groups::get_group_users::{get_group_users_query, GetGroupUsersQueryView};
use crate::endpoints::v1::groups::id::users::get::view::GetGroupUsersResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum GetUsersGroupError {
    BadRequest,
    DatabaseError,
    UnknowGroup,
}

impl std::fmt::Display for GetUsersGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUsersGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database")
            }
            GetUsersGroupError::BadRequest => {
                write!(f, "Bad request")
            }
            GetUsersGroupError::UnknowGroup => {
                write!(f, "Unknow group")
            }
        }
    }
}

impl ResponseError for GetUsersGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUsersGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetUsersGroupError::BadRequest => StatusCode::BAD_REQUEST,
            GetUsersGroupError::UnknowGroup => StatusCode::NOT_FOUND,
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

    let check_view = DoesGroupExistQuerView::new(group_id as u64);
    let result = does_group_exist_query(check_view, pool.clone())
        .await
        .map_err(|_| GetUsersGroupError::UnknowGroup)?;
    if !result {
        return Err(GetUsersGroupError::UnknowGroup);
    }

    let view = GetGroupUsersQueryView::new(group_id as u64);
    let result = get_group_users_query(view, pool)
        .await
        .map_err(|_| GetUsersGroupError::BadRequest)?;

    Ok(result.into())
}

#[utoipa::path(
    get,
    path = "",
    params(
        ("group_id" = u64, Path, description = "ID du groupe")
    ),
    responses(
        (status = 200, body = GetGroupUsersResultView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Unknow group"),
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
