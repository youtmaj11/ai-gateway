use async_trait::async_trait;
use crate::encryption::age::AgeEncryption;
use crate::storage::{ConversationRecord, Storage, StorageError};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

pub struct PostgresStorage {
    client: Client,
    encryption: AgeEncryption,
}

impl PostgresStorage {
    pub async fn new(database_url: &str, encryption: AgeEncryption) -> Result<Self, StorageError> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                eprintln!("Postgres connection error: {err}");
            }
        });

        Ok(Self { client, encryption })
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn query_history(
        &self,
        search: &str,
        since: Option<String>,
    ) -> Result<Vec<ConversationRecord>, StorageError> {
        let mut query = String::from(
            "SELECT user_message, assistant_response, created_at::text AS created_at FROM conversations",
        );

        if since.is_some() {
            query.push_str(" WHERE created_at >= $1");
        }

        query.push_str(" ORDER BY created_at DESC");

        let rows = if let Some(since_dt) = since {
            self.client
                .query(query.as_str(), &[&since_dt])
                .await?
        } else {
            self.client.query(query.as_str(), &[]).await?
        };

        let mut decrypted_records = Vec::new();
        for row in rows {
            let user_message: String = row.try_get("user_message")?;
            let assistant_response: String = row.try_get("assistant_response")?;
            let created_at: String = row.try_get("created_at")?;

            let user_message = self.encryption.decrypt(&user_message)
                .map_err(|err| StorageError::Encryption(err.to_string()))?;
            let assistant_response = self.encryption.decrypt(&assistant_response)
                .map_err(|err| StorageError::Encryption(err.to_string()))?;

            if search.is_empty()
                || user_message.to_lowercase().contains(&search.to_lowercase())
                || assistant_response.to_lowercase().contains(&search.to_lowercase())
            {
                decrypted_records.push(ConversationRecord {
                    user_message,
                    assistant_response,
                    created_at,
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
        let id = Uuid::new_v4();

        self.client
            .execute(
                "INSERT INTO conversations (id, user_message, assistant_response, created_at) VALUES ($1, $2, $3, now())",
                &[&id.to_string(), &encrypted_user_message, &encrypted_assistant_response],
            )
            .await?;

        Ok(())
    }
}
