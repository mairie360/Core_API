//! Role prefix of Core's Redis keys (MAIR-267).
//!
//! The platform's Redis ACL only lets the `core-api` user touch `~core-api:*` (plus the shared
//! `revoked:*` list), but `mairie360_api_lib` 1.2.2 sends keys verbatim. Until a lib version that
//! prefixes keys itself is released and bumped (`mairie360/API_lib` MAIR-267), Core prefixes its
//! own keys with [`redis_key`], which mirrors the lib's rule exactly:
//! 1. `REDIS_KEY_PREFIX`, when set and not empty;
//! 2. else the username of `REDIS_URL` (`redis://core-api:<password>@redis:6379`);
//! 3. else `REDIS_USERNAME`;
//! 4. else no prefix (local Redis without ACL): the key is used as is.
//!
//! **Delete this module when the lib is bumped**: the lib then prefixes every key itself, and
//! keeping [`redis_key`] would prefix twice. The shared `revoked:<sid>` keys never go through it.

use mairie360_api_lib::redis::error::RedisError;
use mairie360_api_lib::redis::redis_interface::Redis;

/// Time to live of the forgot-password token and of its e-mail mapping: 30 minutes.
pub const FORGOT_PASSWORD_TTL_SECONDS: u64 = 30 * 60;

/// Time to live of the first-connection token and of its user id mapping: 24 hours (an
/// administrator creates the account and the user may log in for the first time later that day).
pub const FIRST_CONNECTION_TTL_SECONDS: u64 = 24 * 60 * 60;

/// Username in a `redis://user:password@host:port` URL, if any.
fn username_from_url(redis_url: &str) -> Option<String> {
    let (_, rest) = redis_url.split_once("://")?;
    let (userinfo, _) = rest.rsplit_once('@')?;
    let username = userinfo.split(':').next().unwrap_or_default();
    (!username.is_empty()).then(|| username.to_string())
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// Role prefix of Core's keys, resolved from the environment (see the module documentation).
#[must_use]
pub fn key_prefix() -> Option<String> {
    non_empty_env("REDIS_KEY_PREFIX")
        .or_else(|| non_empty_env("REDIS_URL").and_then(|url| username_from_url(&url)))
        .or_else(|| non_empty_env("REDIS_USERNAME"))
}

/// The key to send to Redis for `key`: `<role>:<key>`, or `key` without prefix.
#[must_use]
pub fn redis_key(key: &str) -> String {
    key_prefix().map_or_else(|| key.to_string(), |prefix| format!("{prefix}:{key}"))
}

/// Stores a one-time token mapping under `redis_key(key)` with a time to live.
///
/// Only written if absent, then given a time to live of `ttl_seconds`: lib 1.2.2 has no atomic
/// `SET … EX NX`, so `EXISTS`/`SET` then `EXPIRE`, all granted by the ACL.
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
    let key = redis_key(key);
    redis.secure_set(&key, value).await?;
    redis.expire(&key, ttl_seconds.max(1)).await
}

#[cfg(test)]
mod tests {
    use super::username_from_url;

    #[test]
    fn username_is_read_from_the_url() {
        assert_eq!(
            username_from_url("redis://core-api:s3cret@redis:6379"),
            Some("core-api".to_string())
        );
        assert_eq!(username_from_url("redis://:password@redis:6379"), None);
        assert_eq!(username_from_url("redis://redis:6379"), None);
    }
}
