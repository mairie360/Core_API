use redis::Connection;
use super::add_key;
use super::key_exist;

pub async fn secure_add_key(conn: &mut Connection, key: &str, value: &str) -> Result<(), redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(true) => add_key(conn, key, value).await,
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "Key does not exist",
            format!("Key '{}' does not exist", key),
        ))),
        Err(err) => Err(err),
    }
}
