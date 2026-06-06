use crate::database::groups::does_group_exist::{does_group_exist_query, DoesGroupExistQuerView};
use crate::database::groups::get_group::{get_group_query, GetGroupQuerView};
use crate::endpoints::v1::groups::id::get::view::GetGroupResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum GetGroupError {
    BadRequest,
    DatabaseError,
    UnknowGroup,
}

impl std::fmt::Display for GetGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetGroupError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            GetGroupError::BadRequest => {
                write!(f, "Bad request.")
            }
            GetGroupError::UnknowGroup => {
                write!(f, "Unknow group.")
            }
        }
    }
}

impl ResponseError for GetGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetGroupError::BadRequest => StatusCode::BAD_REQUEST,
            GetGroupError::UnknowGroup => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_group(
    state: web::Data<AppState>,
    id: u64,
) -> Result<GetGroupResultView, GetGroupError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetGroupError::DatabaseError),
    };

    let group_check_view = DoesGroupExistQuerView::new(id);
    let result = does_group_exist_query(group_check_view, pool.clone())
        .await
        .map_err(|_| GetGroupError::UnknowGroup)?;
    if !result {
        return Err(GetGroupError::UnknowGroup);
    }

    let db_view = GetGroupQuerView::new(id);
    let result = get_group_query(db_view, pool)
        .await
        .map_err(|_| GetGroupError::BadRequest)?;

    Ok(GetGroupResultView::new(result))
}

#[utoipa::path(
    get,
    path = "",
    params(
        ("group_id" = u64, Path, description = "ID du groupe")
    ),
    responses(
        (status = 200, body = GetGroupResultView),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
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
    id: web::Path<u64>,
) -> Result<impl Responder, GetGroupError> {
    let result = get_group(state, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
