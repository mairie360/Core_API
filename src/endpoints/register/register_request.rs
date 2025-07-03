use actix_web::{post, web, HttpResponse, Responder};

use super::super::super::database::db_interface::get_db_interface;
use super::register_view::RegisterView;
use crate::database::queries_result_views::{
    get_boolean_from_query_result, get_result_from_query_result,
};
use crate::database::query_views::{DoesUserExistByEmailQueryView, RegisterUserQueryView};

/**
 * Custom error type for registration errors.
 * This enum defines the possible errors that can occur during user registration.
 * It includes:
 * - `InvalidData`: Indicates that the provided data is invalid.
 * - `UserAlreadyExists`: Indicates that a user with the provided email already exists.
 * - `DatabaseError`: Indicates that there was an error interacting with the database.
 */
#[derive(Debug, Clone, PartialEq)]
enum RegisterError {
    InvalidData,
    UserAlreadyExists,
    DatabaseError,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::InvalidData => write!(f, "Invalid data provided"),
            RegisterError::UserAlreadyExists => write!(f, "User already exists"),
            RegisterError::DatabaseError => write!(f, "Database error occurred"),
        }
    }
}

/**
 * Checks if an email is valid.
 *
 * # Arguments:
 * - `email`: A string representing the email address.
 * # Returns:
 * - `true` if the email is valid.
 * - `false` if the email is empty or does not contain a valid domain.
 */
fn is_valid_email(email: String) -> bool {
    //Need to be more complex and based on requirements
    if email.is_empty() {
        return false;
    }
    match email.find('@') {
        Some(index) => {
            let domain = &email[index + 1..];
            !domain.is_empty() && domain.contains('.')
        }
        None => false,
    }
}

/**
 * Checks if a password is valid.
 *
 * # Arguments:
 * - `password`: A string representing the password.
 * # Returns:
 * - `true` if the password is valid.
 * - `false` if the password does not meet the criteria.
 */
fn is_valid_password(password: String) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

/**
 * Checks if a phone number is valid.
 *
 * # Arguments:
 * - `phone_number`: An optional string representing the phone number.
 * # Returns:
 * - `true` if the phone number is valid or if it is `None`.
 * - `false` if the phone number is provided and does not meet the criteria.
 */
fn is_valid_phone_number(phone_number: Option<String>) -> bool {
    //Need to be more complex and based on requirements
    match phone_number {
        Some(num) => num.len() >= 10 && num.chars().all(|c| c.is_digit(10)),
        None => true,
    }
}

/**
 * Checks if a user already exists in the database.
 *
 * # Arguments:
 * - `register_view`: A reference to a `RegisterView` containing the user's email.
 * # Returns:
 * - `true` if the user already exists.
 * - `false` if the user does not exist or if there is an error during the query.
 */
async fn already_exists(register_view: &RegisterView) -> bool {
    let view = DoesUserExistByEmailQueryView::new(register_view.email());
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return true;
        }
    };
    let query_view = db_interface.execute_query(Box::new(view)).await;
    match query_view {
        Ok(result) => get_boolean_from_query_result(result.get_result()),
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            true
        }
    }
}

/**
 * Checks if the user can be registered.
 * This function validates the provided user data,
 * checks if the user already exists, and returns an appropriate error if any checks fail.
 *
 * # Arguments:
 * - `register_view`: A reference to a `RegisterView` containing user details.
 * # Returns:
 * - `Ok(())` if the user can be registered.
 * - `Err(RegisterError)` if there is an error, such as invalid data or user already exists.
 */
async fn can_be_registered(register_view: &RegisterView) -> Result<(), RegisterError> {
    if !is_valid_email(register_view.email()) {
        return Err(RegisterError::InvalidData);
    }
    if already_exists(&register_view).await {
        return Err(RegisterError::UserAlreadyExists);
    }
    if !is_valid_password(register_view.password()) {
        return Err(RegisterError::InvalidData);
    }
    if !is_valid_phone_number(register_view.phone_number()) {
        return Err(RegisterError::InvalidData);
    }
    Ok(())
}

/**
 * Registers a new user in the system.
 * This function validates the provided user data,
 * checks if the user already exists, and registers the user if all checks pass.
 *
 * # Arguments:
 * - `register_view`: A reference to a `RegisterView` containing user details.
 * # Returns:
 * - `Ok(())` if the user is registered successfully.
 * - `Err(RegisterError)` if there is an error during registration, such as invalid data,
 *   user already exists, or a database error.
 */
async fn register_user(register_view: &RegisterView) -> Result<(), RegisterError> {
    match can_be_registered(register_view).await {
        Ok(_) => {
            let view = RegisterUserQueryView::new(
                register_view.first_name(),
                register_view.last_name(),
                register_view.email(),
                register_view.password(),
                register_view.phone_number(),
            );
            let db_guard = get_db_interface().lock().unwrap();
            let db_interface = match &*db_guard {
                Some(db) => db,
                None => {
                    eprintln!("Database interface is not initialized.");
                    return Err(RegisterError::DatabaseError);
                }
            };
            let query_view = db_interface.execute_query(Box::new(view)).await;
            match query_view {
                Ok(result) => match get_result_from_query_result(result.get_result()) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        eprintln!("Error processing query result: {}", e);
                        Err(RegisterError::DatabaseError)
                    }
                },
                Err(e) => {
                    eprintln!("Error executing query: {}", e);
                    Err(RegisterError::DatabaseError)
                }
            }
        }
        Err(e) => {
            eprintln!("Validation error: {:?}", e);
            Err(e)
        }
    }
}

/**
 * Endpoint to register a new user.
 * This endpoint accepts a JSON payload containing user details,
 * validates the data, checks if the user already exists,
 * and registers the user if all checks pass.
 *
 * # Returns:
 * - `201 Created` if the user is registered successfully.
 * - `400 Bad Request` if the provided data is invalid.
 * - `409 Conflict` if the user already exists.
 * - `500 Internal Server Error` if there is a database error.
 */
#[utoipa::path(
    post,
    path = "/register",
    request_body = RegisterView,
    responses(
        (status = 201, description = "User registered successfully", body = String),
        (status = 400, description = "Invalid data provided", body = String),
        (status = 409, description = "User already exists", body = String),
        (status = 500, description = "Database error occurred", body = String)
    ),
    tag = "Authentication"
)]
#[post("/register")]
pub async fn register(payload: web::Json<RegisterView>) -> impl Responder {
    let register_view = payload.into_inner();
    match register_user(&register_view).await {
        Ok(_) => HttpResponse::Created().body("User registered successfully!"),
        Err(RegisterError::InvalidData) => {
            HttpResponse::BadRequest().body("Invalid data provided.")
        }
        Err(RegisterError::UserAlreadyExists) => {
            HttpResponse::Conflict().body("User already exists.")
        }
        Err(RegisterError::DatabaseError) => {
            HttpResponse::InternalServerError().body("Database error occurred.")
        }
    }
}
