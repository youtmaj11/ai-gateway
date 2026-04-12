use serde::Deserialize;
use std::env;

fn default_port() -> u16 {
    8080
}

fn default_string() -> String {
    String::new()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_string")]
    pub database_url: String,
    #[serde(default = "default_string")]
    pub redis_url: String,
    #[serde(default = "default_string")]
    pub rabbitmq_url: String,
    #[serde(default = "default_string")]
    pub ollama_url: String,
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
