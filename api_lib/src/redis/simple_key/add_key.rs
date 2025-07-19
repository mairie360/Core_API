use redis::{Commands, Connection};

pub async fn add_key(conn: &mut Connection, key: &str, value: &str) -> Result<(), redis::RedisError> {
    match conn.set_nx::<&str, &str, bool>(key, value) {
        Ok(true) => Ok(()),
        Ok(false) => Err(redis::RedisError::from((
            redis::ErrorKind::BusyLoadingError,
            "Key already exists",
        ))),
        Err(err) => Err(err),
    }
}
