use super::about_request_view::{AboutPathParamRequestView, AboutRequestView};

use actix_web::{get, web, HttpRequest, HttpResponse};

use api_lib::database::db_interface::get_db_interface;
use api_lib::database::queries_result_views::get_json_from_query_result;
use api_lib::database::query_views::AboutUserQueryView;
use api_lib::database::utils::does_user_exist_by_id;

use api_lib::redis::redis_manager::get_redis_manager;

use api_macro_lib::check_jwt;

use serde_json;

#[derive(Debug, Clone, PartialEq)]
enum AboutError {
    InvalidCredentials,
    DatabaseError,
}

impl std::fmt::Display for AboutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AboutError::InvalidCredentials => write!(f, "Invalid credentials provided."),
            AboutError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

fn is_response_view_correct(json: &serde_json::Value) -> bool {
    match json {
        serde_json::Value::Object(map) => map.values().all(|v| match v {
            serde_json::Value::String(s) => !s.trim().is_empty(),
            _ => false,
        }),
        _ => false,
    }
}

async fn get_chache_value(
    user_id: u64,
) -> Result<serde_json::Value, AboutError> {
    let mut redis_manager = get_redis_manager().await;
    match redis_manager.as_mut() {
        Some(redis) => {
            let json = redis
                .secure_get_key(&format!("user:{}:about", user_id))
                .await;
            match json {
                Ok(json_str) => {
                    println!("Cached user about info: {}", json_str);
                    serde_json::from_str::<serde_json::Value>(&json_str)
                        .map_err(|_| AboutError::InvalidCredentials)
                }
                Err(e) => {
                    eprintln!("Failed to retrieve cached user about info from Redis: {}", e);
                    Err(AboutError::DatabaseError)
                }
            }
        }
        None => {
            eprintln!("Redis manager is not available.");
            Err(AboutError::DatabaseError)
        }
    }
}

async fn set_cache_value(
    user_id: u64,
    json: &serde_json::Value,
) {
    let mut redis_manager = get_redis_manager().await;
    match redis_manager.as_mut() {
        Some(redis) => {
            let json_str = json.to_string();
            let key_str = format!("user:{}:about", user_id);
            if let Err(e) = redis.secure_add_key(&key_str, &json_str).await {
                eprintln!("Failed to cache user about info in Redis: {}", e);
            }
        }
        None => {
            eprintln!("Redis manager is not available.");
        }
    }
}

async fn about_request(about_view: &AboutRequestView) -> Result<serde_json::Value, AboutError> {
    if !does_user_exist_by_id(about_view.user_id()).await {
        eprintln!("User with ID {} does not exist.", about_view.user_id());
        return Err(AboutError::InvalidCredentials);
    }
    let cached_value = get_chache_value(about_view.user_id()).await;
    match cached_value {
        Ok(json) => {
            if is_response_view_correct(&json) {
                return Ok(json);
            } else {
                eprintln!("Cached response view is not correct: {:?}", json);
            }
        }
        Err(e) => {
            eprintln!("Error retrieving cached value: {}", e);
            // Continue to fetch from the database
        }
    }
    let view = AboutUserQueryView::new(about_view.user_id());
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(AboutError::DatabaseError);
        }
    };
    let query_view = db_interface.execute_query(Box::new(view)).await;
    match query_view {
        Ok(view) => {
            let user_info = view.get_result();
            let json = get_json_from_query_result(user_info);
            if !is_response_view_correct(&json) {
                eprintln!("Response view is not correct: {:?}", json);
                return Err(AboutError::InvalidCredentials);
            }
            set_cache_value(about_view.user_id(), &json).await;
            Ok(json)
        }
        Err(e) => {
            eprintln!("Database error occurred: {}", e);
            Err(AboutError::DatabaseError)
        }
    }
}

#[utoipa::path(
    get,
    path = "/user/${user_id}/about",
    responses(
        (status = 200, description = "User login successfully", body = String),
        (status = 401, description = "Invalid user ID."),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
#[get("/user/{user_id}/about")]
#[check_jwt]
pub async fn user_about(
    req: HttpRequest,
    path_view: web::Path<AboutPathParamRequestView>,
) -> impl Responder {
    let about_view = path_view.into_inner();

    match about_request(&AboutRequestView::new(about_view.user_id())).await {
        Ok(json) => HttpResponse::Ok().body(json.to_string()),
        Err(AboutError::InvalidCredentials) => {
            eprintln!("Invalid credentials provided during about request.");
            HttpResponse::Unauthorized().body("Invalid user ID.")
        }
        Err(AboutError::DatabaseError) => {
            eprintln!("Database error occurred during about request.");
            HttpResponse::InternalServerError().body("Internal server error.")
        }
    }
}
