//! Password hashing for `users.password`.
//!
//! Every write path stores an argon2id PHC string (`hash_password`), never the plaintext value.
//! `is_hashed_password` lets `login` tell a pre-hashing legacy row apart from one already
//! migrated, so accounts created before this module existed keep working: `login` falls back to
//! a plaintext comparison for those rows and re-hashes on success (see
//! `endpoints/v1/auth/login/endpoint.rs`).

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

const ARGON2ID_PREFIX: &str = "$argon2id$";

/// Hashes `password` into an argon2id PHC string, with a fresh random salt.
///
/// # Errors
///
/// Returns an error if argon2 fails to hash the password (should not happen with the default
/// parameters and a fresh salt).
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

/// Whether `stored` already is an argon2id hash, as opposed to a legacy plaintext password.
#[must_use]
pub fn is_hashed_password(stored: &str) -> bool {
    stored.starts_with(ARGON2ID_PREFIX)
}

/// Verifies `password` against a previously computed argon2id hash.
///
/// Returns `false` (never panics) if `hash` is not a valid PHC string.
#[must_use]
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|parsed_hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_password, is_hashed_password, verify_password};

    #[test]
    fn hash_then_verify_succeeds() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(is_hashed_password(&hash));
        assert!(verify_password("correct horse battery staple", &hash));
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(!verify_password("wrong password", &hash));
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        assert!(!verify_password("anything", "not-a-hash"));
    }

    #[test]
    fn is_hashed_password_detects_plaintext() {
        assert!(!is_hashed_password("plaintext123"));
        assert!(!is_hashed_password(""));
    }

    #[test]
    fn each_hash_uses_a_unique_salt() {
        let a = hash_password("same-password").unwrap();
        let b = hash_password("same-password").unwrap();
        assert_ne!(a, b);
    }
}
