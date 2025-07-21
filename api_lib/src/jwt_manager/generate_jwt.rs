use super::get_jwt_secret::get_jwt_secret;
use super::get_jwt_timeout::get_jwt_timeout;
use super::jwt_claims::Claims;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * This function generates a Json Web Token (JWT) for a given user ID.
 * It retrieves the JWT secret and timeout from the environment variables,
 * creates a new Claims object with the user ID and expiration time,
 * and encodes it into a JWT token.
 * If any of the environment variables are not set or invalid, it returns an error.
 * # Arguments
 * * `user_id_str` - A string slice that holds the user ID to be included in the JWT.
 * # Returns
 * * `Result<String, jsonwebtoken::errors::Error>` - A result that contains
 *   the generated JWT token if successful, or an error if generation fails.
 */
pub fn generate_jwt(user_id_str: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret: Vec<u8> = get_jwt_secret().map_err(|_e| {
        jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat)
    })?;
    let timeout = get_jwt_timeout().map_err(|_e| {
        jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)
    });
    if timeout.is_err() {
        panic!("JWT_TIMEOUT environment variable not set or invalid");
    }
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + timeout.unwrap(); // Token valid for 1 hour

    let claims = Claims::new(user_id_str.to_owned(), expiration);

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )?;
    Ok(token)
}
