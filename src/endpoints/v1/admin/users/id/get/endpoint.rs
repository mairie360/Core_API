use crate::{
    database::admin::get_user::{query::get_user_query, view::AdminGetUserQueryView},
    endpoints::v1::admin::users::id::get::view::GetUserResultView,
};
use actix_web::{error::ResponseError, get, http::StatusCode, web, HttpResponse, Responder};
use mairie360_api_lib::pool::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetUserError {
    DatabaseError,
    UnknownUser,
}

impl std::fmt::Display for GetUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUserError::DatabaseError => write!(f, "Database error occurred"),
            GetUserError::UnknownUser => write!(f, "Unknown user"),
        }
    }
}

impl ResponseError for GetUserError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetUserError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetUserError::UnknownUser => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user(
    state: web::Data<AppState>,
    user_id: u64,
) -> Result<GetUserResultView, GetUserError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetUserError::DatabaseError),
    };
    let view = AdminGetUserQueryView::new(user_id);

    let result = get_user_query(view, pool).await.map_err(|e| {
        eprintln!("{:?}", e);
        GetUserError::UnknownUser
    })?;

    Ok(result.into())
}

#[utoipa::path(
    get,
    path = "",
    params(
        ("userId" = u64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "User retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Unknown user"),
        (status = 500, description = "Database error occurred")
    ),
    tag = "Admin - Users"
)]
#[get("/")]
pub async fn admin_get_user(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> Result<impl Responder, GetUserError> {
    let result = get_user(state, path.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
