use actix_web::{post, web, HttpResponse, Responder};

use super::super::super::database::db_interface::get_db_interface;
use super::login_view::LoginView;
use crate::database::queries_result_views::get_u64_from_query_result;
use crate::database::query_views::LoginUserQueryView;
use crate::jwt_manager::generate_jwt::generate_jwt;

#[derive(Debug, Clone, PartialEq)]
enum LoginError {
    InvalidCredentials,
    DatabaseError,
    TokenGenerationError,
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::InvalidCredentials => write!(f, "Invalid credentials provided."),
            LoginError::DatabaseError => {
                write!(f, "An error occurred while accessing the database.")
            }
            LoginError::TokenGenerationError => write!(f, "Failed to generate JWT token."),
        }
    }
}

async fn login_user(login_view: &LoginView) -> Result<(), LoginError> {
    let view = LoginUserQueryView::new(login_view.email(), login_view.password());
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(LoginError::DatabaseError);
        }
    };
    let query_view = db_interface.execute_query(Box::new(view)).await;
    match query_view {
        Ok(result) => {
            let user_id = get_u64_from_query_result(result.get_result());
            if user_id == 0 {
                eprintln!("Login failed: User not found or invalid credentials.");
                return Err(LoginError::InvalidCredentials);
            }
            let jwt = generate_jwt(user_id.to_string().as_str());
            match jwt {
                Ok(token) => println!("Generated JWT: {}", token),
                Err(e) => {
                    eprintln!("Error generating JWT: {}", e);
                    return Err(LoginError::TokenGenerationError);
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(LoginError::DatabaseError)
        }
    }
}

#[post("/login")]
async fn login(payload: web::Json<LoginView>) -> impl Responder {
    let login_view = payload.into_inner();
    match login_user(&login_view).await {
        Ok(_) => HttpResponse::Ok().body("User login successfully!"),
        Err(LoginError::InvalidCredentials) => {
            eprintln!("Invalid credentials provided during login.");
            HttpResponse::Unauthorized().body("Invalid email or password.")
        }
        Err(LoginError::DatabaseError) => {
            eprintln!("Database error occurred during login.");
            HttpResponse::InternalServerError().body("Internal server error.")
        }
        Err(LoginError::TokenGenerationError) => {
            eprintln!("Token generation error occurred during login.");
            HttpResponse::InternalServerError().body("Failed to generate JWT token.")
        }
    }
}
