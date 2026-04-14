use serde::Deserialize;
use std::env;

fn default_port() -> u16 {
    8080
}

fn default_database_url() -> String {
    "sqlite:ai_gateway.db".to_string()
}

fn default_storage_backend() -> StorageBackend {
    StorageBackend::Sqlite
}

fn default_queue_backend() -> QueueBackend {
    QueueBackend::InMemory
}

fn default_string() -> String {
    String::new()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_rate_limit() -> u32 {
    100
}

fn default_rate_limit_window() -> u64 {
    60
}

fn default_encryption_key() -> Option<String> {
    None
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Sqlite,
    Postgres,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum QueueBackend {
    InMemory,
    RabbitMQ,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_storage_backend")]
    pub storage_backend: StorageBackend,
    #[serde(default = "default_encryption_key")]
    pub encryption_key: Option<String>,
    #[serde(default = "default_queue_backend")]
    pub queue_backend: QueueBackend,
    #[serde(default = "default_string")]
    pub redis_url: String,
    #[serde(default = "default_string")]
    pub jwt_secret: String,
    #[serde(default = "default_string")]
    pub rabbitmq_url: String,
    #[serde(default = "default_string")]
    pub ollama_url: String,
    #[serde(default = "default_string")]
    pub homelab_url: String,
    #[serde(default = "default_string")]
    pub homelab_jwt: String,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
    #[serde(default = "default_rate_limit_window")]
    pub rate_limit_window: u64,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, ::config::ConfigError> {
        let settings = ::config::Config::builder()
            .add_source(::config::File::with_name(path).required(false))
            .add_source(::config::Environment::with_prefix("AI_GATEWAY").separator("_"));

        let settings = settings.build()?;
        let mut config: Config = settings.try_deserialize()?;

        if config.log_level.is_empty() {
            config.log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        }

        Ok(config)
    }
}
