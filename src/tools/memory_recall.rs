use std::fmt;

use crate::storage::{get_storage, ConversationRecord};
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

impl MemoryRecallTool {
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
        let storage = get_storage().ok_or_else(|| {
            MemoryRecallError::Request("storage backend is not initialized".to_string())
        })?;

        let records = storage
            .query_history(search, since)
            .await
            .map_err(|err| MemoryRecallError::Request(err.to_string()))?;

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
