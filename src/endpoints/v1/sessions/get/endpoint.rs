use crate::database::sessions::get_active_sessions::{
    get_active_sessions_query, GetActiveSessionsQueryView,
};
use crate::endpoints::v1::sessions::get::response_view::GetResponseView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum GetError {
    DatabaseError,
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        match self {
            GetError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_info(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<GetResponseView, GetError> {
    let user_id = user.id;

    // Le cache Redis est désormais géré par `SmartDatabase` (cache-aside), via
    // `GetActiveSessionsQueryView::cache_key`.
    let query_result = get_active_sessions_query(
        GetActiveSessionsQueryView::new(user_id),
        state.get_smart_db(),
    )
    .await
    .map_err(|_| GetError::DatabaseError)?;

    Ok(GetResponseView::new(
        query_result.into_iter().map(|s| s.into()).collect(),
    ))
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "User info retrieved successfully", body = GetResponseView),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[get("/")]
pub async fn get_active_sessions(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, GetError> {
    let response = get_user_info(user, state).await?;
    Ok(HttpResponse::Ok().json(response))
}
