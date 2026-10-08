//! Argon2 hashing and verification off the async workers (MAIR-474).
//!
//! An argon2id hash costs tens of milliseconds of CPU on purpose. Run inline in a handler, it
//! blocks the actix worker thread and every request queued on it: under the k6 login rush, routes
//! that never touch a password slowed down several times over. These wrappers run the work on
//! actix's blocking thread pool (`web::block`) and must be used instead of calling
//! `mairie360_api_lib::password::{hash_password, verify_password}` from async code.

use actix_web::web;
use mairie360_api_lib::password::{error::PasswordError, hash_password, verify_password};

/// Argon2id hash of `password`.
///
/// # Errors
///
/// [`PasswordError::HashFailed`] when hashing fails or the blocking pool is shut down.
pub async fn hash(password: &str) -> Result<String, PasswordError> {
    let password = password.to_owned();
    web::block(move || hash_password(&password))
        .await
        .map_err(|e| {
            tracing::error!("Password hashing task failed: {e}");
            PasswordError::HashFailed
        })?
}

/// Whether `password` matches the argon2id PHC string `hash`.
///
/// # Errors
///
/// [`PasswordError::InvalidHash`] when `hash` is not a valid PHC string, and
/// [`PasswordError::HashFailed`] when the blocking pool is shut down.
pub async fn verify(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let (password, hash) = (password.to_owned(), hash.to_owned());
    web::block(move || verify_password(&password, &hash))
        .await
        .map_err(|e| {
            tracing::error!("Password verification task failed: {e}");
            PasswordError::HashFailed
        })?
}
