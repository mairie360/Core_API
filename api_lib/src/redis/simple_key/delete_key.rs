use redis::{Commands, Connection};

pub async fn delete_key(conn: &mut Connection, key: &str) -> Result<(), redis::RedisError> {
    match conn.del(key) {
        Ok(0) => Err(redis::RedisError::from((
            redis::ErrorKind::ResponseError,
            "Key not found",
        ))),
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}
