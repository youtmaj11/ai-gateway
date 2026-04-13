use crate::config::Config;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(config: &Config) -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
}
