use actix_web::{post, web, HttpResponse, Responder};

use super::super::super::database::db_interface::get_db_interface;
use super::login_view::LoginView;
use crate::database::queries_result_views::get_u64_from_query_result;
use crate::database::query_views::LoginUserQueryView;
use crate::jwt_manager::generate_jwt::generate_jwt;

/**
 * Enum representing possible errors during user login.
 * This enum is used to handle different error scenarios that can occur
 * when a user attempts to log in, such as invalid credentials, database errors,
 * or issues with JWT token generation.
 *
 * InvalidCredentials: Indicates that the provided email or password is incorrect.
 * DatabaseError: Indicates that there was an issue accessing the database,
 * such as a connection failure or query execution error.
 * TokenGenerationError: Indicates that there was an error while generating the JWT token
 * for the user after a successful login.
 */
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

/**
 * Asynchronously logs in a user by validating their credentials.
 * This function takes a reference to a LoginView object containing the user's email and password,
 * and attempts to authenticate the user against the database.
 * If the credentials are valid, it generates a JWT token for the user.
 * If the credentials are invalid, it returns an InvalidCredentials error.
 * If there is a database error, it returns a DatabaseError.
 *
 * # Arguments
 * * `login_view` - A reference to a LoginView object containing the user's email and password.
 * # Returns
 * * `Result<String, LoginError>` - On success, returns a JWT token as a `String`.
 * On failure, returns a `LoginError` indicating the type of error encountered.
 */
async fn login_user(login_view: &LoginView) -> Result<String, LoginError> {
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
                Ok(token) => Ok(token),
                Err(e) => {
                    eprintln!("Error generating JWT: {}", e);
                    return Err(LoginError::TokenGenerationError);
                }
            }
        }
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            Err(LoginError::DatabaseError)
        }
    }
}

/**
 * Asynchronous handler for user login requests.
 * This function processes incoming login requests, validates the user's credentials,
 * and returns an appropriate HTTP response.
 * If the login is successful, it returns a 200 OK response with a JWT token in the Authorization header.
 * If the login fails due to invalid credentials, it returns a 401 Unauthorized response.
 * If there is a database error or an error generating the JWT token, it returns a 500 Internal Server Error response.
 */
#[post("/login")]
async fn login(payload: web::Json<LoginView>) -> impl Responder {
    let login_view = payload.into_inner();
    match login_user(&login_view).await {
        Ok(jwt) => HttpResponse::Ok()
            .append_header(("Authorization", format!("Bearer {}", jwt)))
            .body("User login successfully!"),
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
