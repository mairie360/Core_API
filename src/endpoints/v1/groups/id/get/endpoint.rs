use crate::endpoints::v1::groups::id::get::view::GetGroupResultView;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::AppState;
use mairie360_api_lib::security::AuthenticatedUser;

#[derive(Debug, Clone, PartialEq)]
enum GetGroupError {
    BadRequest,
    DatabaseError,
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
        }
    }
}

impl ResponseError for GetGroupError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetGroupError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            GetGroupError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_group(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
    id: u64,
) -> Result<(), GetGroupError> {
    let pool = match state.db_pool.clone() {
        Some(pool) => pool,
        None => return Err(GetGroupError::DatabaseError),
    };

    Ok(())
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, body = GetGroupResultView),
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
    id: web::Path<u64>,
) -> Result<impl Responder, GetGroupError> {
    let result = get_group(user, state, id.into_inner()).await?;
    Ok(HttpResponse::Ok().body(result))
}
