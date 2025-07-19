use redis::{Commands, Connection};

pub fn get_key(conn: &mut Connection, key: &str) -> Result<String, redis::RedisError> {
    match conn.get::<&str, String>(key) {
        Ok(value) => Ok(value),
        Err(err) => Err(err),
    }
}
