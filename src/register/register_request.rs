use actix_web::{
    HttpResponse,
    post,
    Responder,
    web
};

use crate::database::query_views::{DoesUserExistByEmailQueryView, RegisterUserQueryView};
use super::super::database::db_interface::get_db_interface;
use super::register_view::RegisterView;
use crate::database::queries_result_views::{get_boolean_from_query_result, get_result_from_query_result};

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

fn is_valid_email(email: String) -> bool {
    //Need to be more complex and based on requirements
    if email.is_empty() {
        return false;
    }
    match email.find('@') {
        Some(index) => {
            let domain = &email[index + 1..];
            !domain.is_empty() && domain.contains('.')
        },
        None => false,
    }
}

fn is_valid_password(password: String) -> bool {
    //Need to be more complex and based on requirements
    password.len() >= 8
}

fn is_valid_phone_number(phone_number: Option<String>) -> bool {
    //Need to be more complex and based on requirements
    match phone_number {
        Some(num) => num.len() >= 10 && num.chars().all(|c| c.is_digit(10)),
        None => true,
    }
}

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
        Ok(result) => {
            get_boolean_from_query_result(result.get_result())
        },
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            true
        }
    }
}

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

async fn register_user(register_view: &RegisterView) -> Result<(), RegisterError> {
    match can_be_registered(register_view).await {
        Ok(_) => {
            let view = RegisterUserQueryView::new(
                register_view.first_name(),
                register_view.last_name(),
                register_view.email(),
                register_view.password(),
                register_view.phone_number()
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
                Ok(result) => {
                    match get_result_from_query_result(result.get_result()) {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            eprintln!("Error processing query result: {}", e);
                            Err(RegisterError::DatabaseError)
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Error executing query: {}", e);
                    Err(RegisterError::DatabaseError)
                }
            }
        },
        Err(e) => {
            eprintln!("Validation error: {:?}", e);
            Err(e)
        }
    }
}

#[post("/register")]
async fn register(payload: web::Json<RegisterView>) -> impl Responder {
    let register_view = payload.into_inner();
    match register_user(&register_view).await {
        Ok(_) => {
            return HttpResponse::Created().body("User registered successfully!");
        },
        Err(RegisterError::InvalidData) => {
            return HttpResponse::BadRequest().body("Invalid data provided.");
        },
        Err(RegisterError::UserAlreadyExists) => {
            return HttpResponse::Conflict().body("User already exists.");
        },
        Err(RegisterError::DatabaseError) => {
            return HttpResponse::InternalServerError().body("Database error occurred.");
        }
    }
}