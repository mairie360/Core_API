use actix_web::{get, web, HttpRequest, HttpResponse};
use super::about_request_view::{AboutPathParamRequestView, AboutRequestView};
use api_macro_lib::check_jwt;
use api_lib::database::db_interface::get_db_interface;
use api_lib::database::query_views::AboutUserQueryView;
use api_lib::database::queries_result_views::get_json_from_query_result;
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
        serde_json::Value::Object(map) => {
            map.values().all(|v| match v {
                serde_json::Value::String(s) => !s.trim().is_empty(),
                _ => false,
            })
        }
        _ => false,
    }
}

async fn about_request(about_view: &AboutRequestView) -> Result<serde_json::Value, AboutError> {
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
pub async fn user_about(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> impl Responder {
    let about_view = path_view.into_inner();
    println!("About request view: {:}", about_view);

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
