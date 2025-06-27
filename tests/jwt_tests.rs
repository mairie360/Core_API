use core_api::jwt_manager::generate_jwt::generate_jwt;
use core_api::jwt_manager::get_jwt_secret::get_jwt_secret;
use core_api::jwt_manager::get_user_id_from_jwt::get_user_id_from_jwt;
use once_cell::sync::Lazy;
use std::env;

static USER_ID: &str = "1";

static INIT: Lazy<()> = Lazy::new(|| {
    // This code runs ONCE before any test
    env::set_var("JWT_SECRET", "b\"secret\"");
    env::set_var("JWT_TIMEOUT", "3600");
    println!("Global setup done");
});

fn setup() {
    // Force INIT to run
    Lazy::force(&INIT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_jwt_secret() {
        setup();
        let secret = get_jwt_secret();
        println!("secret: {:?}", secret);
        assert!(
            secret.is_ok(),
            "Failed to get JWT secret: {:?}",
            secret.err()
        );
        assert_eq!(
            secret.unwrap(),
            "b\"secret\"".to_string().into_bytes(),
            "JWT secret does not match expected value"
        );
    }

    #[test]
    fn test_get_jwt_timeout() {
        setup();
        let timeout = core_api::jwt_manager::get_jwt_timeout::get_jwt_timeout();
        assert!(
            timeout.is_ok(),
            "Failed to get JWT timeout: {:?}",
            timeout.err()
        );
        assert_eq!(
            timeout.unwrap(),
            3600,
            "JWT timeout does not match expected value"
        );
    }

    #[test]
    fn test_generate_jwt() {
        setup();
        let token = generate_jwt(USER_ID);
        assert!(token.is_ok(), "JWT generation failed: {:?}", token.err());
        let token = token.unwrap();
        assert!(!token.is_empty(), "Generated JWT token is empty");
    }

    #[test]
    fn test_get_user_id_from_jwt() {
        setup();
        let token = generate_jwt(USER_ID).unwrap();
        let user_id = get_user_id_from_jwt(&token);
        assert_eq!(
            user_id.unwrap(),
            USER_ID,
            "User ID does not match expected value"
        );
    }

    #[test]
    fn test_get_user_id_from_invalid_jwt() {
        setup();
        let invalid_token = "invalid.token.string";
        let user_id = get_user_id_from_jwt(invalid_token);
        assert_eq!(user_id, None, "Expected None for invalid JWT, got Some");
    }
}
