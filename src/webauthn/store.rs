use super::CHALLENGE_TTL_SECONDS;
use crate::redis_keys::set_token;
use mairie360_api_lib::redis::error::RedisError;
use mairie360_api_lib::redis::redis_interface::Redis;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::{DiscoverableAuthentication, PasskeyRegistration};

/// Registration ceremony waiting for the authenticator's answer, bound to the account that
/// started it: another account cannot finish it.
#[derive(Serialize, Deserialize)]
pub struct PendingRegistration {
    pub user_id: u64,
    pub state: PasskeyRegistration,
}

/// Authentication ceremony waiting for the assertion. Discoverable: no account is known yet,
/// the credential the authenticator answers with names it.
#[derive(Serialize, Deserialize)]
pub struct PendingAuthentication {
    pub state: DiscoverableAuthentication,
}

/// Pending ceremonies, in Redis (keys prefixed by the lib like every Core key, MAIR-267).
///
/// A state is written once (`SET … EX NX`, [`CHALLENGE_TTL_SECONDS`]) and read once: `take_*`
/// deletes it, so a replayed answer finds nothing and is refused.
pub struct ChallengeStore<'a> {
    redis: &'a Redis,
}

impl<'a> ChallengeStore<'a> {
    #[must_use]
    pub const fn new(redis: &'a Redis) -> Self {
        Self { redis }
    }

    /// Stores a registration ceremony under a fresh challenge id, returned.
    ///
    /// # Errors
    ///
    /// [`RedisError`] when the state cannot be serialised or Redis refuses the write.
    pub async fn store_registration(
        &self,
        pending: &PendingRegistration,
    ) -> Result<Uuid, RedisError> {
        self.store(&registration_key(Uuid::new_v4()), pending).await
    }

    /// Takes a registration ceremony back, once: `None` when unknown, expired or already used.
    ///
    /// # Errors
    ///
    /// [`RedisError`] when Redis cannot be reached.
    pub async fn take_registration(
        &self,
        challenge_id: Uuid,
    ) -> Result<Option<PendingRegistration>, RedisError> {
        self.take(&registration_key(challenge_id)).await
    }

    /// Stores an authentication ceremony under a fresh challenge id, returned.
    ///
    /// # Errors
    ///
    /// [`RedisError`] when the state cannot be serialised or Redis refuses the write.
    pub async fn store_authentication(
        &self,
        pending: &PendingAuthentication,
    ) -> Result<Uuid, RedisError> {
        self.store(&authentication_key(Uuid::new_v4()), pending)
            .await
    }

    /// Takes an authentication ceremony back, once: `None` when unknown, expired or already used.
    ///
    /// # Errors
    ///
    /// [`RedisError`] when Redis cannot be reached.
    pub async fn take_authentication(
        &self,
        challenge_id: Uuid,
    ) -> Result<Option<PendingAuthentication>, RedisError> {
        self.take(&authentication_key(challenge_id)).await
    }

    async fn store<T: Serialize + Sync>(&self, key: &Key, value: &T) -> Result<Uuid, RedisError> {
        let encoded = serde_json::to_string(value)
            .map_err(|e| RedisError::Value(format!("ceremony state: {e}")))?;
        set_token(self.redis, &key.redis_key, &encoded, CHALLENGE_TTL_SECONDS).await?;
        Ok(key.challenge_id)
    }

    async fn take<T: DeserializeOwned>(&self, key: &Key) -> Result<Option<T>, RedisError> {
        let Some(encoded) = self.redis.secure_get::<String>(&key.redis_key).await? else {
            return Ok(None);
        };
        // Single use: whatever the outcome of the ceremony, the challenge is gone.
        self.redis.delete(&key.redis_key).await?;
        // A state Core wrote itself and cannot read back is a bug, not a client error.
        serde_json::from_str(&encoded)
            .map(Some)
            .map_err(|e| RedisError::Value(format!("ceremony state: {e}")))
    }
}

struct Key {
    challenge_id: Uuid,
    redis_key: String,
}

fn registration_key(challenge_id: Uuid) -> Key {
    Key {
        challenge_id,
        redis_key: format!("passkey_registration/{challenge_id}"),
    }
}

fn authentication_key(challenge_id: Uuid) -> Key {
    Key {
        challenge_id,
        redis_key: format!("passkey_authentication/{challenge_id}"),
    }
}
