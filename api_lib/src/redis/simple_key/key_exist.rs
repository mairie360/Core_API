use redis::{Commands, Connection};

pub async fn key_exist(conn: &mut Connection, key: &str) -> Result<bool, redis::RedisError> {
    match conn.exists(key) {
        Ok(exists) => Ok(exists),
        Err(err) => Err(err),
    }
}
