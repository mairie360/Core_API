use redis::Connection;
use super::key_exist;
use super::get_key;

pub async fn secure_get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    match key_exist(conn, key).await {
        Ok(true) => {
            get_key(conn, key).await
        }
        Ok(false) => {
            Err(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Key does not exist",
                format!("Key '{}' does not exist", key),
            )))
        }
        Err(err) => {
            Err(err)
        }
    }
}
