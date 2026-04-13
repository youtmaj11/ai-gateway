use crate::policy::opa::{PolicyEnforcer, UserContext, PolicyError};

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
