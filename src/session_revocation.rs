//! Publishes revoked sessions to the Redis revocation list shared by every API (MAIR-264).
//!
//! When a session ends, Core writes `revoked:<sid>` in Redis, with a time to live equal to the
//! remaining lifetime of the session's JWTs (at most `JWT_TIMEOUT`). Once the other APIs run a
//! `mairie360_api_lib` version whose `JwtMiddleware` checks that key (`mairie360/API_lib#141`),
//! they refuse the JWTs of the session too. Core itself already refuses them through
//! `endpoints::session_guard`, which reads the `sessions` table.
//!
//! Local copy of the lib's `revoke_session`: Core cannot depend on the unreleased lib version.
//! Same key (`revoked:` + the session id, never prefixed), so both implementations are
//! interchangeable; replace this module by the lib helper when the lib is bumped.
//!
//! The lib 1.2.2 `Redis` has no atomic `SET ... EX`, so the key is written with `SET` then
//! `EXPIRE` (both allowed by the Redis ACL of the API roles).

use crate::database::sessions::revoke_user_sessions::RevokeUserSessionsQueryView;
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::jwt_manager::get_jwt_timeout;
use mairie360_api_lib::redis::error::RedisError;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::state::AppState;
use uuid::Uuid;

/// Prefix of the revocation keys, shared by every API (same as the lib).
pub const REVOKED_SESSION_KEY_PREFIX: &str = "revoked:";

/// Redis key marking the session `session_id` as revoked.
#[must_use]
pub fn revoked_session_key(session_id: Uuid) -> String {
    format!("{REVOKED_SESSION_KEY_PREFIX}{session_id}")
}

/// Upper bound of a JWT's lifetime: `JWT_TIMEOUT`, or one hour if it cannot be read.
#[must_use]
pub fn max_jwt_lifetime() -> u64 {
    get_jwt_timeout()
        .ok()
        .and_then(|timeout| u64::try_from(timeout).ok())
        .unwrap_or(3600)
}

/// Marks `session_id` as revoked for `ttl_seconds` (clamped to 1..=`JWT_TIMEOUT`).
///
/// # Errors
///
/// Returns a [`RedisError`] when Redis cannot be reached or rejects a command.
pub async fn publish_revoked_session(
    redis: &Redis,
    session_id: Uuid,
    ttl_seconds: u64,
) -> Result<(), RedisError> {
    let key = revoked_session_key(session_id);
    let ttl = ttl_seconds.clamp(1, max_jwt_lifetime().max(1));
    redis.set(&key, 1_i32).await?;
    redis.expire(&key, ttl).await
}

/// Marks every session of `session_ids` as revoked for `JWT_TIMEOUT`.
///
/// `JWT_TIMEOUT`, because their latest JWT may have been issued just now by a refresh.
/// Failures are logged, not returned: this runs after
/// the revocation is committed in Postgres, which Core itself enforces, so the request that
/// triggered it still succeeds. Returns whether every key was written.
pub async fn publish_revoked_sessions(redis: &Redis, session_ids: &[Uuid]) -> bool {
    let ttl = max_jwt_lifetime();
    let mut all_written = true;
    for &session_id in session_ids {
        if let Err(e) = publish_revoked_session(redis, session_id, ttl).await {
            eprintln!(
                "[CRITICAL] Could not publish revoked session {session_id} to Redis: {e}. \
                 Other APIs accept its JWTs until they expire."
            );
            all_written = false;
        }
    }
    all_written
}

/// Revokes every active session of `user_id` in Postgres and publishes them to the revocation
/// list. Used when the password changes or the account goes away.
///
/// # Errors
///
/// Returns an [`ApiLibError`] when the sessions cannot be revoked in Postgres. Redis failures
/// are only logged (see [`publish_revoked_sessions`]).
pub async fn revoke_all_user_sessions(
    state: &AppState,
    user_id: u64,
) -> Result<Vec<Uuid>, ApiLibError> {
    let revoked: Vec<Uuid> = state
        .get_smart_db()
        .fetch_all(&RevokeUserSessionsQueryView::all(user_id))
        .await?;
    publish_revoked_sessions(state.get_redis(), &revoked).await;
    Ok(revoked)
}
