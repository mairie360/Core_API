//! Time to live of Core's one-time Redis tokens (MAIR-267).
//!
//! The keys themselves are prefixed by `mairie360_api_lib` (`<role>:`, from `REDIS_KEY_PREFIX` or
//! the username of `REDIS_URL`): Core passes bare keys and never adds the prefix itself. The shared
//! `revoked:<sid>` keys are the only ones sent unprefixed, and only the lib writes them.

use mairie360_api_lib::redis::error::RedisError;
use mairie360_api_lib::redis::redis_interface::Redis;

/// Time to live of the forgot-password token and of its e-mail mapping: 30 minutes.
pub const FORGOT_PASSWORD_TTL_SECONDS: u64 = 30 * 60;

/// Time to live of the first-connection token and of its user id mapping: 24 hours (an
/// administrator creates the account and the user may log in for the first time later that day).
pub const FIRST_CONNECTION_TTL_SECONDS: u64 = 24 * 60 * 60;

/// Stores a one-time token mapping with a time to live.
///
/// Written only if absent, atomically with its time to live (`SET … EX NX`): an existing token is
/// left untouched.
///
/// # Errors
///
/// Returns a [`RedisError`] when Redis cannot be reached or refuses a command.
pub async fn set_token(
    redis: &Redis,
    key: &str,
    value: &str,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    redis
        .secure_set_ex(key, value, ttl_seconds.max(1))
        .await
        .map(|_| ())
}
