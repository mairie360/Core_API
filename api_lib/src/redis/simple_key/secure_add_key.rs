use redis::Connection;
use super::add_key;
use super::key_exist;

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
