use crate::get_critical_env_var;
use redis::{Client, Connection};
use std::sync::{LazyLock};
use tokio::sync::Mutex;

static REDIS_INTERFACE: LazyLock<Mutex<Option<RedisManager>>> =
    LazyLock::new(|| Mutex::new(Some(RedisManager::new())));

pub async fn create_redis_manager() {
    let mut guard = REDIS_INTERFACE.lock().await;
    if guard.is_none() {
        *guard = Some(RedisManager::new());
    }
}

pub async fn get_redis_manager() -> tokio::sync::MutexGuard<'static, Option<RedisManager>> {
    REDIS_INTERFACE.lock().await
}

pub struct RedisManager {
    client: Client,
    connection: Option<Connection>,
}

impl RedisManager {
    fn new() -> Self {
        let redis_url = get_critical_env_var("REDIS_URL");
        println!("Connecting to Redis at: {}", redis_url);
        let client = Client::open(redis_url).expect("Failed to create Redis client");
        RedisManager {
            client,
            connection: None,
        }
    }

    pub fn connect(&mut self) -> Result<String, redis::RedisError> {
        self.connection = Some(self.client.get_connection()?);
        match self.connection {
            Some(_) => Ok("Connected to Redis successfully".to_string()),
            None => Err(redis::RedisError::from((redis::ErrorKind::IoError, "Failed to connect to Redis"))),
        }
    }
}
