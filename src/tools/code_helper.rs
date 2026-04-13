use serde_json::json;
use std::env;
use std::fmt;

use crate::tools::Tool;

#[derive(Debug)]
pub enum CodeHelperError {
    Request(String),
    Parse(String),
}

impl fmt::Display for CodeHelperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeHelperError::Request(err) => write!(f, "request failed: {err}"),
            CodeHelperError::Parse(err) => write!(f, "parse failed: {err}"),
        }
    }
}

impl std::error::Error for CodeHelperError {}

pub struct CodeHelperTool;

impl CodeHelperTool {
    fn ollama_url() -> String {
        env::var("AI_GATEWAY_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
    }

    fn build_prompt(instruction: &str, code: &str) -> String {
        if code.is_empty() {
            format!(
                "You are a helpful code assistant.\n\nInstruction: {instruction}\n\nProvide a code-focused response with the generated or refactored code and a short explanation.",
            )
        } else {
            format!(
                "You are a helpful code assistant.\n\nInstruction: {instruction}\n\nOriginal code:\n{code}\n\nGenerate or refactor the code as requested and return the updated code with a short explanation.",
            )
        }
    }

    fn parse_params(params: &str) -> (&str, &str) {
        if let Some((instruction, code)) = params.split_once("\n\n") {
            (instruction.trim(), code.trim())
        } else {
            (params.trim(), "")
        }
    }

    fn query_ollama(prompt: &str) -> Result<String, CodeHelperError> {
        let url = format!("{}/v1/chat", Self::ollama_url());
        let body = json!({
            "model": "llama2",
            "messages": [
                {"role": "system", "content": "You are a code assistant that returns code and explanation."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.0
        });

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
            .map_err(|err| CodeHelperError::Request(err.to_string()))?;

        let text = response
            .into_string()
            .map_err(|err| CodeHelperError::Request(err.to_string()))?;

        Ok(text)
    }
}

impl Tool for CodeHelperTool {
    fn name(&self) -> &'static str {
        "code_helper"
    }

    fn execute(&self, params: &str) -> String {
        let (instruction, code) = Self::parse_params(params);
        let prompt = Self::build_prompt(instruction, code);

        match Self::query_ollama(&prompt) {
            Ok(result) => format!("CodeHelperTool result:\n{result}"),
            Err(err) => format!("CodeHelperTool error: {err}"),
        }
    }
}
