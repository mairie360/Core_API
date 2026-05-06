use crate::database::sessions::get_active_sessions::{
    get_active_sessions_query, GetActiveSessionsQueryView,
};
use crate::database::sessions::Session;
use crate::endpoints::v1::sessions::get::response_view::GetResponseView;
use mairie360_api_lib::security::AuthenticatedUser;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};
use mairie360_api_lib::pool::redis::simple_key::secured::{handle_secure_get, handle_secure_post};
use mairie360_api_lib::pool::AppState;
use serde_json;

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

// --- Cache Logic ---

async fn get_cache_value(user_id: u64, state: &web::Data<AppState>) -> Option<GetResponseView> {
    match state.get_redis_conn().await {
        Some(redis_manager) => {
            let key = format!("sessions:{}", user_id);
            if let Ok(json_str) = handle_secure_get(redis_manager, &key).await {
                // La désérialisation vers la struct valide automatiquement le format
                serde_json::from_str::<GetResponseView>(&json_str).ok()
            } else {
                None
            }
        }
        None => None,
    }
}

async fn set_cache_value(user_id: u64, data: &Vec<Session>, state: &web::Data<AppState>) {
    match state.get_redis_conn().await {
        Some(redis_manager) => {
            if let Ok(json_str) = serde_json::to_string(data) {
                let key = format!("sessions:{}", user_id);
                let _ = handle_secure_post(redis_manager, &key, &json_str).await;
            }
        }
        None => {}
    }
}

async fn get_user_info(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<GetResponseView, GetError> {
    let user_id = user.id;

    // 1. Tentative via Cache
    if let Some(cached) = get_cache_value(user_id, &state).await {
        return Ok(cached);
    }

    // 2. Récupération depuis la base de données
    let query_result = get_active_sessions_query(
        GetActiveSessionsQueryView::new(user_id),
        state.db_pool.clone().unwrap(),
    )
    .await
    .map_err(|_| GetError::DatabaseError)?;

    // 3. Mise en cache
    set_cache_value(user_id, &query_result, &state).await;

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
pub async fn get(
    user: AuthenticatedUser,
    state: web::Data<AppState>,
) -> Result<impl Responder, GetError> {
    let response = get_user_info(user, state).await?;
    Ok(HttpResponse::Ok().json(response))
}
