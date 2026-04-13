use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;
use std::fmt;

use crate::tools::Tool;

#[derive(Debug)]
pub enum MemoryRecallError {
    Request(String),
    Parse(String),
}

impl fmt::Display for MemoryRecallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryRecallError::Request(err) => write!(f, "request failed: {err}"),
            MemoryRecallError::Parse(err) => write!(f, "parse failed: {err}"),
        }
    }
}

impl std::error::Error for MemoryRecallError {}

pub struct MemoryRecallTool;

struct ConversationRecord {
    user_message: String,
    assistant_response: String,
    created_at: String,
}

impl MemoryRecallTool {
    fn database_url() -> Result<String, MemoryRecallError> {
        env::var("AI_GATEWAY_DATABASE_URL")
            .map_err(|err| MemoryRecallError::Request(err.to_string()))
    }

    fn parse_params(params: &str) -> (String, Option<String>) {
        let trimmed = params.trim();
        if let Some((keyword, since_part)) = trimmed.rsplit_once(" since:") {
            let keyword = keyword.trim().to_string();
            let since = since_part.trim().to_string();
            (keyword, Some(since))
        } else {
            (trimmed.to_string(), None)
        }
    }

    async fn query_history(
        search: &str,
        since: Option<String>,
    ) -> Result<Vec<ConversationRecord>, MemoryRecallError> {
        let database_url = Self::database_url()?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .map_err(|err| MemoryRecallError::Request(err.to_string()))?;

        let mut query = String::from(
            "SELECT user_message, assistant_response, created_at::text AS created_at FROM conversations \
             WHERE (user_message ILIKE $1 OR assistant_response ILIKE $1)",
        );

        if since.is_some() {
            query.push_str(" AND created_at >= $2");
        }

        query.push_str(" ORDER BY created_at DESC LIMIT 8");

        let search_pattern = format!("%{}%", search);
        let records = if let Some(since_dt) = since {
            sqlx::query_as::<_, ConversationRecord>(&query)
                .bind(search_pattern)
                .bind(since_dt)
                .fetch_all(&pool)
                .await
                .map_err(|err| MemoryRecallError::Request(err.to_string()))?
        } else {
            sqlx::query_as::<_, ConversationRecord>(&query)
                .bind(search_pattern)
                .fetch_all(&pool)
                .await
                .map_err(|err| MemoryRecallError::Request(err.to_string()))?
        };

        Ok(records)
    }

    fn format_results(results: &[ConversationRecord]) -> String {
        if results.is_empty() {
            return "No matching conversations found.".to_string();
        }

        let mut output = String::from("Found memory recall matches:\n\n");
        for (index, record) in results.iter().enumerate() {
            output.push_str(&format!(
                "{}. [{}] User: {}\n   Assistant: {}\n\n",
                index + 1,
                record.created_at,
                record.user_message,
                record.assistant_response,
            ));
        }
        output.push_str("Return this information to the agent as relevant context.");
        output
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for ConversationRecord {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            user_message: row.try_get("user_message")?,
            assistant_response: row.try_get("assistant_response")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl Tool for MemoryRecallTool {
    fn name(&self) -> &'static str {
        "memory_recall"
    }

    fn execute(&self, params: &str) -> String {
        let (keyword, since) = Self::parse_params(params);
        let results = tokio::runtime::Handle::current().block_on(Self::query_history(&keyword, since));

        match results {
            Ok(records) => Self::format_results(&records),
            Err(err) => format!("MemoryRecallTool error: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_params_with_since() {
        let params = "payment issue since:2025-01-01";
        let (keyword, since) = MemoryRecallTool::parse_params(params);

        assert_eq!(keyword, "payment issue");
        assert_eq!(since, Some("2025-01-01".to_string()));
    }

    #[test]
    fn parse_params_without_since() {
        let params = "database migration error";
        let (keyword, since) = MemoryRecallTool::parse_params(params);

        assert_eq!(keyword, "database migration error");
        assert!(since.is_none());
    }
}
