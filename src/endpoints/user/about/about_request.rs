use actix_web::{get, web, HttpRequest, HttpResponse};
use super::about_request_view::{AboutPathParamRequestView, AboutRequestView};
use api_macro_lib::check_jwt;
// use super::about_response_view::AboutResponseView;
// use api_lib::database::db_interface::get_db_interface;
use api_lib::database::query_views::AboutUserQueryView;
use api_lib::database::db_interface::get_db_interface;
use api_lib::database::queries_result_views::get_boolean_from_query_result;
use api_lib::database::query_views::DoesUserExistByIdQueryView;
use api_lib::jwt_manager::verify_jwt_timeout;
use api_lib::jwt_manager::get_jwt_from_request;
use api_lib::jwt_manager::get_timeout_from_jwt;
use api_lib::jwt_manager::get_user_id_from_jwt;

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

async fn about_request(about_view: &AboutRequestView) -> Result<(), AboutError> {
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
    Ok(())
}

#[utoipa::path(
    get,
    path = "/user/${user_id}/about",
    responses(
        (status = 200, description = "User login successfully", body = String),
        (status = 401, description = "Invalid email or password"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
#[get("/user/{user_id}/about")]
#[check_jwt]
pub async fn user_about(req: HttpRequest, path_view: web::Path<AboutPathParamRequestView>) -> impl Responder {
    //print about request view
    let about_view = path_view.into_inner();
    println!("About request view: {:}", about_view);

    match about_request(&AboutRequestView::new(about_view.user_id())).await {
        Ok(_) => HttpResponse::Ok().body("User about information retrieved successfully."),
        Err(AboutError::InvalidCredentials) => {
            eprintln!("Invalid credentials provided during about request.");
            HttpResponse::Unauthorized().body("Invalid user ID.")
        }
        Err(AboutError::DatabaseError) => {
            eprintln!("Database error occurred during about request.");
            HttpResponse::InternalServerError().body("Internal server error.")
        }
    }

//     match login_user(&login_view).await {
//         Ok(jwt) => HttpResponse::Ok()
//             .append_header(("Authorization", format!("Bearer {}", jwt)))
//             .body("User login successfully!"),
//         Err(LoginError::InvalidCredentials) => {
//             eprintln!("Invalid credentials provided during login.");
//             HttpResponse::Unauthorized().body("Invalid email or password.")
//         }
//         Err(LoginError::DatabaseError) => {
//             eprintln!("Database error occurred during login.");
//             HttpResponse::InternalServerError().body("Internal server error.")
//         }
//         Err(LoginError::TokenGenerationError) => {
//             eprintln!("Token generation error occurred during login.");
//             HttpResponse::InternalServerError().body("Failed to generate JWT token.")
//         }
//     }
}
