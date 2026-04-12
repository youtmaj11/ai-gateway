use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub rabbitmq_url: String,
    pub ollama_url: String,
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
