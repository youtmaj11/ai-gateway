use serde_json::Value;
use std::env;
use std::fmt;

use crate::tools::Tool;
use urlencoding::encode;

#[derive(Debug)]
pub enum WebSearchError {
    Request(String),
    Parse(String),
}

impl fmt::Display for WebSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebSearchError::Request(err) => write!(f, "request failed: {err}"),
            WebSearchError::Parse(err) => write!(f, "result parse failed: {err}"),
        }
    }
}

impl std::error::Error for WebSearchError {}

pub struct WebSearchTool;

impl WebSearchTool {
    fn base_url() -> String {
        env::var("AI_GATEWAY_SEARX_URL").unwrap_or_else(|_| "http://localhost:8888".to_string())
    }

    fn format_results(source: &str, query: &str, results: &[SearchResult]) -> String {
        if results.is_empty() {
            return format!("No search results returned from {source} for query: '{query}'");
        }

        let mut output = format!("Search results from {source} for query: '{query}':\n\n");
        for (index, item) in results.iter().take(5).enumerate() {
            output.push_str(&format!(
                "{}. Title: {title}\n   URL: {url}\n   Summary: {snippet}\n\n",
                index + 1,
                title = item.title,
                url = item.url,
                snippet = item.snippet,
            ));
        }

        output
    }

    fn parse_searx_results(body: &str) -> Result<Vec<SearchResult>, WebSearchError> {
        let json: Value = serde_json::from_str(body)
            .map_err(|err| WebSearchError::Parse(err.to_string()))?;

        let results = json
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| WebSearchError::Parse("missing results array".to_string()))?;

        let parsed = results
            .iter()
            .map(|item| SearchResult {
                title: item
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("(no title)")
                    .to_string(),
                url: item
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or("(no url)")
                    .to_string(),
                snippet: item
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("(no summary)")
                    .to_string(),
            })
            .collect();

        Ok(parsed)
    }

    fn parse_duckduckgo_results(body: &str) -> Result<Vec<SearchResult>, WebSearchError> {
        let json: Value = serde_json::from_str(body)
            .map_err(|err| WebSearchError::Parse(err.to_string()))?;

        let related = json
            .get("RelatedTopics")
            .and_then(Value::as_array)
            .ok_or_else(|| WebSearchError::Parse("missing RelatedTopics array".to_string()))?;

        let mut parsed = Vec::new();
        for item in related.iter() {
            if let Some(text) = item.get("Text").and_then(Value::as_str) {
                let url = item
                    .get("FirstURL")
                    .and_then(Value::as_str)
                    .unwrap_or("(no url)")
                    .to_string();
                parsed.push(SearchResult {
                    title: text.to_string(),
                    url,
                    snippet: text.to_string(),
                });
            } else if let Some(topics) = item.get("Topics").and_then(Value::as_array) {
                for topic in topics.iter() {
                    if let Some(text) = topic.get("Text").and_then(Value::as_str) {
                        let url = topic
                            .get("FirstURL")
                            .and_then(Value::as_str)
                            .unwrap_or("(no url)")
                            .to_string();
                        parsed.push(SearchResult {
                            title: text.to_string(),
                            url,
                            snippet: text.to_string(),
                        });
                    }
                }
            }
        }

        Ok(parsed)
    }

    fn query_searx(query: &str) -> Result<Vec<SearchResult>, WebSearchError> {
        let base = Self::base_url();
        let encoded = encode(query);
        let url = format!("{base}/search.json?q={encoded}&format=json");
        let response = ureq::get(&url)
            .set("Accept", "application/json")
            .call()
            .map_err(|err| WebSearchError::Request(err.to_string()))?;

        let body = response
            .into_string()
            .map_err(|err| WebSearchError::Request(err.to_string()))?;

        Self::parse_searx_results(&body)
    }

    fn query_duckduckgo(query: &str) -> Result<Vec<SearchResult>, WebSearchError> {
        let encoded = encode(query);
        let url = format!("https://api.duckduckgo.com/?q={encoded}&format=json&no_redirect=1&skip_disambig=1");
        let response = ureq::get(&url)
            .set("Accept", "application/json")
            .call()
            .map_err(|err| WebSearchError::Request(err.to_string()))?;

        let body = response
            .into_string()
            .map_err(|err| WebSearchError::Request(err.to_string()))?;

        Self::parse_duckduckgo_results(&body)
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn execute(&self, params: &str) -> String {
        let query = params.trim();
        if query.is_empty() {
            return "Usage: web_search <query>".to_string();
        }

        match Self::query_searx(query) {
            Ok(results) if !results.is_empty() => Self::format_results("SearxNG", query, &results),
            Ok(_) => match Self::query_duckduckgo(query) {
                Ok(results) => Self::format_results("DuckDuckGo", query, &results),
                Err(err) => format!("No search results available: {err}"),
            },
            Err(_) => match Self::query_duckduckgo(query) {
                Ok(results) => Self::format_results("DuckDuckGo", query, &results),
                Err(err) => format!("Search unavailable: {err}"),
            },
        }
    }
}
