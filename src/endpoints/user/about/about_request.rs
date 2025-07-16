use actix_web::{get, web, HttpRequest, HttpResponse, Responder};

use super::about_request_view::{AboutPathParamRequestView, AboutRequestView};
use api_macro_lib::check_jwt;
// use super::about_response_view::AboutResponseView;
// use crate::database::db_interface::get_db_interface;
// use crate::database::query_views::LoginUserQueryView;
use api_lib::jwt_manager::get_jwt_from_request::get_jwt_from_request;

#[derive(Debug, Clone, PartialEq)]
enum LoginError {
    InvalidCredentials,
    DatabaseError,
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials provided."),
            LoginError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
        }
    }
}

async fn get_user_about(login_view: &AboutRequestView) -> Result<(), LoginError> {
    // let view = LoginUserQueryView::new(login_view.email(), login_view.password());
    // let db_guard = get_db_interface().lock().unwrap();
    // let db_interface = match &*db_guard {
    //     Some(db) => db,
    //     None => {
    //         eprintln!("Database interface is not initialized.");
    //         return Err(LoginError::DatabaseError);
    //     }
    // };
    // let query_view = db_interface.execute_query(Box::new(view)).await;
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
    // let user_id = path_view.user_id();
    // let jwt = match get_jwt_from_request(&req) {
    //     Some(token) => token,
    //     None => {
    //         eprintln!("No JWT token found in the request.");
    //         return HttpResponse::Unauthorized().body("Unauthorized: No JWT token provided.");
    //     }
    // };
    // let login_view = AboutRequestView::new(user_id, jwt.to_string());
    // println!("login_view: {}", login_view);

    HttpResponse::Ok().finish()

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
