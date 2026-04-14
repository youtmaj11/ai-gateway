use async_trait::async_trait;
use crate::encryption::age::AgeEncryption;
use crate::storage::{ConversationRecord, Storage, StorageError};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

pub struct SqliteStorage {
    pool: SqlitePool,
    encryption: AgeEncryption,
}

impl SqliteStorage {
    pub async fn new(database_url: &str, encryption: AgeEncryption) -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool, encryption })
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
            "SELECT user_message, assistant_response, created_at FROM conversations",
        );

        if since.is_some() {
            query.push_str(" WHERE created_at >= ?1");
        }

        query.push_str(" ORDER BY created_at DESC");

        let records = if let Some(since_dt) = since {
            sqlx::query_as::<_, ConversationRecord>(&query)
                .bind(since_dt)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, ConversationRecord>(&query)
                .fetch_all(&self.pool)
                .await?
        };

        let mut decrypted_records = Vec::new();
        for record in records {
            let user_message = self.encryption.decrypt(&record.user_message)
                .map_err(|err| StorageError::Encryption(err.to_string()))?;
            let assistant_response = self.encryption.decrypt(&record.assistant_response)
                .map_err(|err| StorageError::Encryption(err.to_string()))?;

            if search.is_empty()
                || user_message.to_lowercase().contains(&search.to_lowercase())
                || assistant_response.to_lowercase().contains(&search.to_lowercase())
            {
                decrypted_records.push(ConversationRecord {
                    user_message,
                    assistant_response,
                    created_at: record.created_at,
                });
            }
        }

        Ok(decrypted_records.into_iter().take(8).collect())
    }

    async fn save_conversation(
        &self,
        user_message: &str,
        assistant_response: &str,
    ) -> Result<(), StorageError> {
        let encrypted_user_message = self.encryption.encrypt(user_message)
            .map_err(|err| StorageError::Encryption(err.to_string()))?;
        let encrypted_assistant_response = self.encryption.encrypt(assistant_response)
            .map_err(|err| StorageError::Encryption(err.to_string()))?;
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO conversations (id, user_message, assistant_response, created_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(id)
        .bind(encrypted_user_message)
        .bind(encrypted_assistant_response)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
