use crate::get_critical_env_var;
use redis::{Client, Connection};
use std::sync::LazyLock;
use super::simple_key;
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
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to connect to Redis",
            ))),
        }
    }

    pub async fn set_key(&mut self, key: &str, value: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::set_key(conn, key, value).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }

   pub async fn add_key(&mut self, key: &str, value: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::add_key(conn, key, value).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }

    pub async fn get_key(&mut self, key: &str) -> Result<String, redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::get_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }

    pub async fn secure_get_key(&mut self, key: &str) -> Result<String, redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::secure_get_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }

    pub async fn delete_key(&mut self, key: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::delete_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }

    pub async fn secure_delete_key(&mut self, key: &str) -> Result<(), redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::secure_delete_key(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }

    pub async fn key_exist(&mut self, key: &str) -> Result<bool, redis::RedisError> {
        match &mut self.connection {
            Some(conn) => simple_key::key_exist(conn, key).await,
            None => Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "No Redis connection established",
            ))),
        }
    }
}
