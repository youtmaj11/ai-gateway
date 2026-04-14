use async_trait::async_trait;
use crate::encryption::age::AgeEncryption;
use crate::storage::{ConversationRecord, Storage, StorageError};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
    encryption: AgeEncryption,
}

impl SqliteStorage {
    pub async fn new(database_url: &str, encryption: AgeEncryption) -> Result<Self, StorageError> {
        let database_url = database_url.to_string();
        let connection = tokio::task::spawn_blocking(move || Connection::open(database_url))
            .await
            .map_err(|err| StorageError::Database(err.to_string()))??;

        Ok(Self {
            conn: Arc::new(Mutex::new(connection)),
            encryption,
        })
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

        let conn = self.conn.clone();
        let query = query.clone();
        let since_clone = since.clone();

        let rows = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let mut stmt = conn.prepare(&query)?;
            let mut rows = if let Some(since_dt) = since_clone {
                stmt.query(params![since_dt])?
            } else {
                stmt.query([])?
            };

            let mut records = Vec::new();
            while let Some(row) = rows.next()? {
                records.push((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ));
            }

            Ok::<_, rusqlite::Error>(records)
        })
        .await
        .map_err(|err| StorageError::Database(err.to_string()))??;

        let mut decrypted_records = Vec::new();
        for (user_message, assistant_response, created_at) in rows {
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
                    created_at: created_at.clone(),
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

        let conn = self.conn.clone();
        let encrypted_user_message = encrypted_user_message.clone();
        let encrypted_assistant_response = encrypted_assistant_response.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            conn.execute(
                "INSERT INTO conversations (id, user_message, assistant_response, created_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
                params![id, encrypted_user_message, encrypted_assistant_response],
            )?;
            Ok::<_, rusqlite::Error>(())
        })
        .await
        .map_err(|err| StorageError::Database(err.to_string()))??;

        Ok(())
    }
}
