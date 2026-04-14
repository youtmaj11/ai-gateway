pub mod postgres;
pub mod redis;
pub mod sqlite;

use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::{fmt, sync::Arc};

use crate::config::{Config, StorageBackend};
use crate::encryption::age::AgeEncryption;

pub use postgres::PostgresStorage;
pub use sqlite::SqliteStorage;

static GLOBAL_STORAGE: OnceCell<Arc<dyn Storage>> = OnceCell::new();

#[derive(Debug)]
pub enum StorageError {
    Database(String),
    Encryption(String),
    AlreadyInitialized,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Database(err) => write!(f, "storage error: {err}"),
            StorageError::Encryption(err) => write!(f, "storage encryption error: {err}"),
            StorageError::AlreadyInitialized => write!(f, "storage backend already initialized"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<tokio_postgres::Error> for StorageError {
    fn from(err: tokio_postgres::Error) -> Self {
        StorageError::Database(err.to_string())
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        StorageError::Database(err.to_string())
    }
}

pub fn get_storage() -> Option<&'static Arc<dyn Storage>> {
    GLOBAL_STORAGE.get()
}

pub async fn initialize_storage(config: &Config) -> Result<(), StorageError> {
    let storage = create_storage(config).await?;
    GLOBAL_STORAGE
        .set(storage)
        .map_err(|_| StorageError::AlreadyInitialized)
}

pub async fn create_storage(config: &Config) -> Result<Arc<dyn Storage>, StorageError> {
    let encryption = AgeEncryption::new(config.encryption_key.clone());

    let backend: Arc<dyn Storage> = match &config.storage_backend {
        StorageBackend::Sqlite => Arc::new(SqliteStorage::new(&config.database_url, encryption.clone()).await?),
        StorageBackend::Postgres => Arc::new(PostgresStorage::new(&config.database_url, encryption.clone()).await?),
    };

    Ok(backend)
}

#[derive(Debug, Clone)]
pub struct ConversationRecord {
    pub user_message: String,
    pub assistant_response: String,
    pub created_at: String,
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn query_history(
        &self,
        search: &str,
        since: Option<String>,
    ) -> Result<Vec<ConversationRecord>, StorageError>;

    async fn save_conversation(
        &self,
        user_message: &str,
        assistant_response: &str,
    ) -> Result<(), StorageError>;
}
