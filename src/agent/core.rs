use crate::queue::Queue;
use crate::storage::{get_storage, redis::RedisCache};
use crate::tools::{run_tool, ToolRegistry};
use serde::Deserialize;
use serde_json::json;
use std::env;
use tokio::sync::mpsc;

/// Agent core helper functions for the CLI and future runtime.
pub async fn run_chat(
    message: &str,
    mut cache: Option<&mut RedisCache>,
    queue: &(dyn Queue + Send + Sync),
) -> String {
    let key = format!("chat:{}", message);

    if let Some(cache) = cache.as_mut() {
        if let Ok(Some(cached)) = cache.get(&key).await {
            return format!("Cached response: {}", cached);
        }
    }

    let task = message.to_string();
    let queued = queue.enqueue(task.clone()).await;
    let queued_response = match queued {
        Ok(()) => match queue.dequeue().await {
            Ok(Some(processed)) => format!("Processed queued task: {}", processed),
            Ok(None) => format!("Queued task but no task was received: {}", task),
            Err(err) => format!("Queue error: {err}"),
        },
        Err(err) => format!("Queue error: {err}"),
    };

    if let Some(storage) = get_storage() {
        let _ = storage
            .save_conversation(&message, &queued_response)
            .await;
    }

    if let Some(cache) = cache.as_mut() {
        let _ = cache.set(&key, &queued_response, 60).await;
    }

    queued_response
}

pub struct AgentCore;

impl AgentCore {
    pub async fn run_agent(prompt: &str) -> String {
        let mut conversation = format!("User: {prompt}\n");
        let mut attempts = 0;

        while attempts < 4 {
            let query = Self::build_ollama_prompt(&conversation);
            let reply = match Self::send_to_ollama(&query).await {
                Ok(reply) => reply,
                Err(err) => return format!("Agent error: {err}"),
            };

            match Self::parse_agent_response(&reply) {
                AgentAction::ToolCall { tool_name, tool_input } => {
                    let tool_output = Self::execute_tool(&tool_name, &tool_input).await;
                    conversation.push_str(&format!(
                        "Tool call: {tool_name}\nInput: {tool_input}\nResult: {tool_output}\n",
                    ));
                }
                AgentAction::FinalAnswer(answer) => {
                    return answer;
                }
                AgentAction::Unknown(raw) => {
                    return format!("Final answer: {raw}");
                }
            }

            attempts += 1;
        }

        format!(
            "Agent stopped after {attempts} iterations. Conversation:\n{conversation}"
        )
    }

    pub async fn run_agent_stream(
        prompt: &str,
        sender: mpsc::UnboundedSender<String>,
    ) -> String {
        let mut conversation = format!("User: {prompt}\n");
        let mut attempts = 0;

        while attempts < 4 {
            let step = attempts + 1;
            let _ = sender.send(format!("Thinking: step {step} started"));

            let query = Self::build_ollama_prompt(&conversation);
            let _ = sender.send("Thinking: querying Ollama".to_string());

            let reply = match Self::send_to_ollama(&query).await {
                Ok(reply) => {
                    let _ = sender.send(format!("Thinking: received Ollama response"));
                    reply
                }
                Err(err) => {
                    let error = format!("Agent error: {err}");
                    let _ = sender.send(error.clone());
                    return error;
                }
            };

            match Self::parse_agent_response(&reply) {
                AgentAction::ToolCall { tool_name, tool_input } => {
                    let _ = sender.send(format!("Planning: call tool {tool_name}"));
                    let tool_output = Self::execute_tool(&tool_name, &tool_input).await;
                    let _ = sender.send(format!(
                        "Tool result: {tool_name} => {tool_output}",
                    ));

                    conversation.push_str(&format!(
                        "Tool call: {tool_name}\nInput: {tool_input}\nResult: {tool_output}\n",
                    ));
                }
                AgentAction::FinalAnswer(answer) => {
                    let _ = sender.send(format!("Final answer: {answer}"));
                    return answer;
                }
                AgentAction::Unknown(raw) => {
                    let result = format!("Final answer: {raw}");
                    let _ = sender.send(result.clone());
                    return result;
                }
            }

            attempts += 1;
        }

        let final_message = format!(
            "Agent stopped after {attempts} iterations. Conversation:\n{conversation}"
        );
        let _ = sender.send(final_message.clone());
        final_message
    }

    async fn execute_tool(tool_name: &str, tool_input: &str) -> String {
        if tool_name == "shell_executor" {
            match run_tool(tool_name, tool_input, "agent", "admin").await {
                Ok(result) => result,
                Err(err) => format!("tool execution failed: {err}"),
            }
        } else {
            let registry = ToolRegistry::new();

            match registry.get(tool_name) {
                Some(tool) => tool.execute(tool_input),
                None => format!("tool not found: {tool_name}"),
            }
        }
    }

    fn build_ollama_prompt(conversation: &str) -> String {
        format!(
            "You are an agent controller that either returns a structured JSON tool call or a final answer.\n\n{conversation}\nRespond with JSON in one of these forms:\n{{\"action\":\"call_tool\",\"tool_name\":\"<tool>\",\"tool_input\":\"<input>\"}}\nor\n{{\"action\":\"final_answer\",\"answer\":\"<answer>\"}}\nDo not include any extra text outside the JSON."
        )
    }

    async fn send_to_ollama(prompt: &str) -> Result<String, String> {
        let url = format!("{}/v1/chat", Self::ollama_url());
        let prompt = prompt.to_string();

        tokio::task::spawn_blocking(move || {
            let body = json!({
                "model": "llama2",
                "messages": [
                    {"role": "system", "content": "You are an agent assistant that plans tool usage."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.0
            });

            let response = ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&body.to_string())
                .map_err(|err| err.to_string())?;

            response.into_string().map_err(|err| err.to_string())
        })
        .await
        .map_err(|err| err.to_string())?
    }

    fn ollama_url() -> String {
        env::var("AI_GATEWAY_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
    }

    fn parse_agent_response(response: &str) -> AgentAction {
        if let Some(json_text) = Self::extract_json(response) {
            if let Ok(parsed) = serde_json::from_str::<AgentResponse>(&json_text) {
                return match parsed.action.as_str() {
                    "call_tool" => AgentAction::ToolCall {
                        tool_name: parsed.tool_name.unwrap_or_default(),
                        tool_input: parsed.tool_input.unwrap_or_default(),
                    },
                    "final_answer" => AgentAction::FinalAnswer(parsed.answer.unwrap_or_default()),
                    _ => AgentAction::Unknown(response.to_string()),
                };
            }
        }

        AgentAction::Unknown(response.to_string())
    }

    fn extract_json(response: &str) -> Option<String> {
        let start = response.find('{')?;
        let end = response.rfind('}')?;
        if end > start {
            Some(response[start..=end].to_string())
        } else {
            None
        }
    }
}

#[derive(Deserialize)]
struct AgentResponse {
    action: String,
    tool_name: Option<String>,
    tool_input: Option<String>,
    answer: Option<String>,
}

enum AgentAction {
    ToolCall { tool_name: String, tool_input: String },
    FinalAnswer(String),
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_call_response() {
        let response = "{\"action\":\"call_tool\",\"tool_name\":\"web_search\",\"tool_input\":\"rust async\"}";
        match AgentCore::parse_agent_response(response) {
            AgentAction::ToolCall { tool_name, tool_input } => {
                assert_eq!(tool_name, "web_search");
                assert_eq!(tool_input, "rust async");
            }
            _ => panic!("Expected tool call action"),
        }
    }

    #[test]
    fn parse_final_answer_response() {
        let response = "{\"action\":\"final_answer\",\"answer\":\"Use the web search tool.\"}";
        match AgentCore::parse_agent_response(response) {
            AgentAction::FinalAnswer(answer) => assert_eq!(answer, "Use the web search tool."),
            _ => panic!("Expected final answer action"),
        }
    }
}
