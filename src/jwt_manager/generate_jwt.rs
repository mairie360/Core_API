use jsonwebtoken::{encode, Header, EncodingKey};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use super::get_jwt_secret::get_jwt_secret;

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn generate_jwt(user_id_str: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret: Vec<u8> = get_jwt_secret().map_err(|_e| jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat))?;
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize + 3600; // Expire dans 1 heure

    let claims = Claims {
        sub: user_id_str.to_owned(),
        exp: expiration,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(&secret))?;
    Ok(token)
}
