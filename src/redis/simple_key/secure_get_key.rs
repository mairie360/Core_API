use redis::Connection;
use super::key_exist;
use super::get_key;

pub fn secure_get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    match key_exist(conn, key) {
        Ok(true) => {
            get_key(conn, key)
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
