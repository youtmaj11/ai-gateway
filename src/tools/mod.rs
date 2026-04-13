use crate::policy::opa::{PolicyEnforcer, UserContext, PolicyError};
use std::fmt;
use std::path::PathBuf;

pub mod file_reader;
pub mod pdf_book_loader;
pub mod shell_executor;
pub mod web_search;
pub mod code_helper;

use code_helper::CodeHelperTool;
use file_reader::FileReaderTool;
use pdf_book_loader::PdfBookLoaderTool;
use shell_executor::ShellExecutorTool;
use web_search::WebSearchTool;

/// Tool execution errors returned when an OPA policy denies action.
#[derive(Debug)]
pub enum ToolExecutionError {
    ToolNotFound(String),
    PolicyDenied(String),
    PolicyError(PolicyError),
}

impl From<PolicyError> for ToolExecutionError {
    fn from(error: PolicyError) -> Self {
        ToolExecutionError::PolicyError(error)
    }
}

impl fmt::Display for ToolExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolExecutionError::ToolNotFound(message) => write!(f, "{message}"),
            ToolExecutionError::PolicyDenied(message) => write!(f, "{message}"),
            ToolExecutionError::PolicyError(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ToolExecutionError {}

pub trait Tool {
    fn name(&self) -> &'static str;
    fn execute(&self, params: &str) -> String;
}

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool + Send + Sync>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: vec![
                Box::new(FileReaderTool),
                Box::new(PdfBookLoaderTool),
                Box::new(ShellExecutorTool),
                Box::new(WebSearchTool),
                Box::new(CodeHelperTool),
            ],
        }
    }

    pub fn get(&self, tool_name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools.iter().find_map(|tool| {
            if tool.name() == tool_name {
                Some(tool.as_ref())
            } else {
                None
            }
        })
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.name()).collect()
    }
}

/// Run a tool command with OPA authorization checks.
pub async fn run_tool(
    tool_name: &str,
    params: &str,
    username: &str,
    role: &str,
) -> Result<String, ToolExecutionError> {
    let policy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("policies/tools.rego");
    let mut enforcer = PolicyEnforcer::load_policy(policy_path)?;

    let user = UserContext {
        username: username.to_string(),
        role: role.to_string(),
    };

    if !enforcer.allow_tool(tool_name, &user)? {
        return Err(ToolExecutionError::PolicyDenied(format!(
            "tool '{tool_name}' not allowed for user role '{user_role}'",
            user_role = user.role
        )));
    }

    let registry = ToolRegistry::new();

    if tool_name == "shell_executor" {
        let output = ShellExecutorTool::execute_async(params).await;
        return Ok(output);
    }

    let tool = registry
        .get(tool_name)
        .ok_or_else(|| ToolExecutionError::ToolNotFound(tool_name.to_string()))?;

    Ok(tool.execute(params))
}

pub fn registered_tools() -> Vec<&'static str> {
    ToolRegistry::new().names()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shell_tool_allowed_for_admin() {
        let result = run_tool("shell", "", "admin", "admin_user").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Executed tool: shell");
    }

    #[tokio::test]
    async fn shell_tool_denied_for_non_admin() {
        let result = run_tool("shell", "", "developer", "developer_user").await;
        assert!(matches!(result, Err(ToolExecutionError::PolicyDenied(_))));
    }

    #[test]
    fn registered_tools_include_pdf_book_loader() {
        let tools = registered_tools();
        assert!(tools.contains(&"pdf_book_loader"));
        assert!(tools.contains(&"shell_executor"));
    }

    #[test]
    fn registered_tools_include_web_search() {
        let tools = registered_tools();
        assert!(tools.contains(&"web_search"));
    }

    #[test]
    fn registered_tools_include_code_helper() {
        let tools = registered_tools();
        assert!(tools.contains(&"code_helper"));
    }
}
