use crate::database::users::about::{about_user_query, AboutUserQueryView};
use crate::endpoints::v1::user::about::about_request_view::{
    AboutPathParamRequestView, AboutRequestView,
};
use crate::endpoints::v1::user::about::about_response_view::AboutResponseView;

use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, Responder, ResponseError};

use mairie360_api_lib::database::queries::does_user_exist_by_id_query;

use mairie360_api_lib::database::query_views::DoesUserExistByIdQueryView;

use mairie360_api_lib::pool::redis::simple_key::secured::{handle_secure_get, handle_secure_post};
use mairie360_api_lib::pool::AppState;
use serde_json;

#[derive(Debug, Clone, PartialEq)]
enum AboutError {
    UserNotFound,
    DatabaseError,
}

impl std::fmt::Display for AboutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AboutError::UserNotFound => write!(f, "User not found."),
            AboutError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

impl ResponseError for AboutError {
    fn status_code(&self) -> StatusCode {
        match self {
            AboutError::UserNotFound => StatusCode::UNAUTHORIZED, // On garde 401 selon tes specs
            AboutError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

// --- Cache Logic ---

async fn get_cache_value(user_id: u64, state: &web::Data<AppState>) -> Option<AboutResponseView> {
    let redis_manager = state.get_redis_conn().await;
    let key = format!("user:{}:about", user_id);

    if let Ok(json_str) = handle_secure_get(redis_manager, &key).await {
        // La désérialisation vers la struct valide automatiquement le format
        return serde_json::from_str::<AboutResponseView>(&json_str).ok();
    }
    None
}

async fn set_cache_value(user_id: u64, data: &AboutResponseView, state: &web::Data<AppState>) {
    let redis_manager = state.get_redis_conn().await;
    if let Ok(json_str) = serde_json::to_string(data) {
        let key = format!("user:{}:about", user_id);
        let _ = handle_secure_post(redis_manager, &key, &json_str).await;
    }
}

async fn about_request(
    about_view: &AboutRequestView,
    state: web::Data<AppState>,
) -> Result<AboutResponseView, AboutError> {
    let user_id = about_view.user_id();

    let exists = does_user_exist_by_id_query(
        DoesUserExistByIdQueryView::new(user_id),
        state.db_pool.clone(),
    )
    .await
    .map_err(|e| {
        eprintln!("Error checking existence for user {}: {}", user_id, e);
        AboutError::DatabaseError
    })?;

    if !exists {
        return Err(AboutError::UserNotFound);
    }

    // 1. Tentative via Cache
    if let Some(cached) = get_cache_value(user_id, &state).await {
        return Ok(cached);
    }

    // 2. Query Database
    let query_result = about_user_query(AboutUserQueryView::new(user_id), state.db_pool.clone())
        .await
        .map_err(|_| AboutError::DatabaseError)?;

    // On transforme le résultat brut en AboutResponseView
    // Si about_user_query renvoie déjà une structure compatible, on l'utilise
    let response = AboutResponseView::from(query_result);

    // 3. Mise en cache et retour
    set_cache_value(user_id, &response, &state).await;
    Ok(response)
}

#[utoipa::path(
    get,
    path = "{user_id}/about",
    responses(
        (status = 200, description = "User info retrieved successfully", body = AboutResponseView),
        (status = 401, description = "Invalid user ID"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("user_id" = u64, Path, description = "The ID of the user"),
    ),
    tag = "Users",
    security(
        ("jwt" = [])
    )
)]
#[get("/{user_id}/about")]
pub async fn about(
    path: web::Path<AboutPathParamRequestView>,
    state: web::Data<AppState>,
) -> Result<impl Responder, AboutError> {
    let about_view = path.into_inner();

    let response_data = about_request(&AboutRequestView::new(about_view.user_id()), state).await?;

    Ok(HttpResponse::Ok().json(response_data))
}
