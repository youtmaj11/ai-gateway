use async_trait::async_trait;
use crate::storage::{ConversationRecord, Storage, StorageError};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn query_history(
        &self,
        search: &str,
        since: Option<String>,
    ) -> Result<Vec<ConversationRecord>, StorageError> {
        let mut query = String::from(
            "SELECT user_message, assistant_response, created_at FROM conversations \
             WHERE (user_message LIKE ?1 OR assistant_response LIKE ?1)",
        );

        if since.is_some() {
            query.push_str(" AND created_at >= ?2");
        }

        query.push_str(" ORDER BY created_at DESC LIMIT 8");
        let search_pattern = format!("%{}%", search);

        let records = if let Some(since_dt) = since {
            sqlx::query_as::<_, ConversationRecord>(&query)
                .bind(search_pattern)
                .bind(since_dt)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, ConversationRecord>(&query)
                .bind(search_pattern)
                .fetch_all(&self.pool)
                .await?
        };

        Ok(records)
    }
}
