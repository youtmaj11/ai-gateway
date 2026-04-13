use crate::policy::opa::{PolicyEnforcer, UserContext, PolicyError};
use std::fmt;
use std::path::PathBuf;

/// Tool execution errors returned when an OPA policy denies action.
#[derive(Debug)]
pub enum ToolExecutionError {
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
            ToolExecutionError::PolicyDenied(message) => write!(f, "{message}"),
            ToolExecutionError::PolicyError(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ToolExecutionError {}

/// Execute a tool request after an OPA policy check.
pub async fn execute_tool(
    tool_name: &str,
    user: &UserContext,
    enforcer: &mut PolicyEnforcer,
) -> Result<String, ToolExecutionError> {
    if !enforcer.allow_tool(tool_name, user)? {
        return Err(ToolExecutionError::PolicyDenied(format!(
            "tool '{tool_name}' not allowed for user role '{user_role}'",
            user_role = user.role
        )));
    }

    Ok(format!("Executed tool: {tool_name}"))
}

/// Run a tool command with OPA authorization checks.
pub async fn run_tool(tool_name: &str, username: &str, role: &str) -> Result<String, ToolExecutionError> {
    let policy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("policies/tools.rego");
    let mut enforcer = PolicyEnforcer::load_policy(policy_path)?;

    let user = UserContext {
        username: username.to_string(),
        role: role.to_string(),
    };

    execute_tool(tool_name, &user, &mut enforcer).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shell_tool_allowed_for_admin() {
        let result = run_tool("shell", "admin_user", "admin").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Executed tool: shell");
    }

    #[tokio::test]
    async fn shell_tool_denied_for_non_admin() {
        let result = run_tool("shell", "developer_user", "developer").await;
        assert!(matches!(result, Err(ToolExecutionError::PolicyDenied(_))));
    }
}
