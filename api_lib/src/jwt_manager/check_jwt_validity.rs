use crate::database::db_interface::get_db_interface;
use crate::database::queries_result_views::get_boolean_from_query_result;
use crate::jwt_manager::verify_jwt_timeout;
use crate::jwt_manager::get_timeout_from_jwt;
use crate::jwt_manager::get_user_id_from_jwt;
use crate::database::query_views::DoesUserExistByIdQueryView;

pub enum JWTCheckError {
    DatabaseError,
    NoTokenProvided,
    ExpiredToken,
    InvalidToken,
    UnknowUser,
}

pub async fn check_jwt_validity(jwt: &str) -> Result<(), JWTCheckError> {
    if jwt.is_empty() {
        eprintln!("No JWT token provided.");
        return Err(JWTCheckError::NoTokenProvided);
    }
    let user_id = match get_user_id_from_jwt(&jwt) {
        Some(id) => id,
        None => {
            eprintln!("Failed to decode JWT token.");
            return Err(JWTCheckError::InvalidToken);
        }
    };
    println!("User ID from JWT: {}", user_id);

    let view = DoesUserExistByIdQueryView::new(user_id.parse().unwrap());
    let db_guard = get_db_interface().lock().unwrap();
    let db_interface = match &*db_guard {
        Some(db) => db,
        None => {
            eprintln!("Database interface is not initialized.");
            return Err(JWTCheckError::DatabaseError);
        }
    };
    let query_view = db_interface.execute_query(Box::new(view)).await;

    let result = match query_view {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error executing query: {}", e);
            return Err(JWTCheckError::DatabaseError);
        }
    };

    if !get_boolean_from_query_result(result.get_result()) {
        eprintln!("User does not exist with ID: {}", user_id);
        return Err(JWTCheckError::UnknowUser);
    }

    let timeout: usize = match get_timeout_from_jwt(&jwt) {
        Some(t) => t,
        None => {
            eprintln!("Failed to retrieve timeout from JWT.");
            return Err(JWTCheckError::InvalidToken);
        }
    };

    match verify_jwt_timeout(timeout) {
        Ok(true) => {
            println!("JWT token is valid and not expired.");
            Ok(())
        },
        Ok(false) => {
            eprintln!("JWT token is expired.");
            Err(JWTCheckError::ExpiredToken)
        },
        Err(e) => {
            eprintln!("Error verifying JWT timeout: {}", e);
            Err(JWTCheckError::InvalidToken)
        }
    }
}
