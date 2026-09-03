use crate::database::sessions::get_sessions_by_user::{
    get_sessions_by_user_query, GetSessionsByUserQueryView,
};
use crate::endpoints::v1::sessions::history::response_view::HistoryResponseView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::state::AppState;

#[derive(Debug, Clone, PartialEq)]
enum HistoryError {
    DatabaseError,
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for HistoryError {
    fn status_code(&self) -> StatusCode {
        match self {
            HistoryError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

async fn get_user_info(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<HistoryResponseView, HistoryError> {
    let user_id = user.id;

    // Le cache Redis est désormais géré par `SmartDatabase` (cache-aside), via
    // `GetSessionsByUserQueryView::cache_key`.
    let query_result = get_sessions_by_user_query(
        GetSessionsByUserQueryView::new(user_id),
        state.get_smart_db(),
    )
    .await
    .map_err(|_| HistoryError::DatabaseError)?;

    Ok(HistoryResponseView::new(
        query_result.into_iter().map(|s| s.into()).collect(),
    ))
}

#[utoipa::path(
    get,
    path = "history",
    responses(
        (status = 200, description = "User info retrieved successfully", body = HistoryResponseView),
        (status = 500, description = "Internal server error")
    ),
    tag = "Sessions",
    security(
        ("jwt" = [])
    )
)]
#[get("/history")]
pub async fn history(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, HistoryError> {
    let result = get_user_info(user, state).await?;
    Ok(HttpResponse::Ok().json(result))
}
