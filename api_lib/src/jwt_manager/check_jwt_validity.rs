use crate::database::utils::does_user_exist_by_id;
use crate::jwt_manager::get_timeout_from_jwt;
use crate::jwt_manager::get_user_id_from_jwt;
use crate::jwt_manager::verify_jwt_timeout;

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

    match does_user_exist_by_id(user_id.parse().unwrap()).await {
        true => println!("User exists with ID: {}", user_id),
        false => {
            eprintln!("User does not exist with ID: {}", user_id);
            return Err(JWTCheckError::UnknowUser);
        }
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
        }
        Ok(false) => {
            eprintln!("JWT token is expired.");
            Err(JWTCheckError::ExpiredToken)
        }
        Err(e) => {
            eprintln!("Error verifying JWT timeout: {}", e);
            Err(JWTCheckError::InvalidToken)
        }
    }
}
