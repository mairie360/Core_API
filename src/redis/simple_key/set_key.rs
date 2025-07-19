use redis::{Commands, Connection};

pub fn set_key(conn: &mut Connection, key: &str, value: &str) -> Result<(), redis::RedisError> {
    match conn.set::<&str, &str, String>(key, value) {
        Ok(response) => {
            if response == "OK" {
                Ok(())
            } else {
                Err(redis::RedisError::from((
                    redis::ErrorKind::ResponseError,
                    "Unexpected SET response",
                )))
            }
        }
        Err(err) => Err(err),
    }
}
