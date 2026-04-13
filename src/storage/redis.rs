use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client, RedisError};

#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
}

impl RedisCache {
    pub async fn new(url: &str) -> Result<Self, RedisError> {
        let client = Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self { manager })
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<String>, RedisError> {
        self.manager.get(key).await
    }

    pub async fn set(&mut self, key: &str, value: &str, ttl_seconds: usize) -> Result<(), RedisError> {
        self.manager.set_ex(key, value, ttl_seconds as u64).await
    }
}
