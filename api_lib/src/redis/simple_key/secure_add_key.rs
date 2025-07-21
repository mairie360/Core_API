use redis::Connection;
use super::add_key;
use super::key_exist;

/**
 * Securely adds a key to Redis if it does not already exist.
 * If the key already exists, it returns an error.
 *
 * # Arguments
 * * `conn` - A mutable reference to the Redis connection.
 * * `key` - The key to be added.
 * * `value` - The value to be associated with the key.
 *
 * # Returns
 * * `Ok(())` if the key was added successfully.
 * * `Err(redis::RedisError)` if the key already exists or if there was an error.
 */
pub async fn secure_add_key(conn: &mut Connection, key: &str, value: &str) -> Result<(), redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(false) => add_key(conn, key, value).await,
        Ok(true) => Err(redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "Key already exist",
            format!("Key '{}' already exist", key),
        ))),
        Err(err) => Err(err),
    }
}
