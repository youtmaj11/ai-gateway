use regorus::Engine;
use serde::Serialize;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct UserContext {
    pub username: String,
    pub role: String,
}

#[derive(Debug)]
pub enum PolicyError {
    Load(String),
    Eval(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyError::Load(err) => write!(f, "Policy load error: {err}"),
            PolicyError::Eval(err) => write!(f, "Policy evaluation error: {err}"),
        }
    }
}

impl std::error::Error for PolicyError {}

pub struct PolicyEnforcer {
    engine: Engine,
}

impl PolicyEnforcer {
    pub fn load_policy<P: AsRef<Path>>(policy_file: P) -> Result<Self, PolicyError> {
        let mut engine = Engine::new();
        engine
            .add_policy_from_file(policy_file)
            .map_err(|err| PolicyError::Load(err.to_string()))?;

        Ok(Self { engine })
    }

    pub fn allow_tool(&mut self, tool_name: &str, user: &UserContext) -> Result<bool, PolicyError> {
        let input = serde_json::json!({
            "tool": tool_name,
            "user": {
                "name": user.username,
                "role": user.role,
            }
        });

        self.engine
            .set_input_json(&input.to_string())
            .map_err(|err| PolicyError::Eval(err.to_string()))?;

        self.engine
            .eval_allow_query("data.tools.allow".to_string(), false)
            .map_err(|err| PolicyError::Eval(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn policy_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("policies/tools.rego")
    }

    #[test]
    fn shell_only_allowed_for_admins() {
        let path = policy_path();
        let mut enforcer = PolicyEnforcer::load_policy(path).expect("failed to load policy");

        let admin = UserContext {
            username: "admin_user".into(),
            role: "admin".into(),
        };

        let developer = UserContext {
            username: "dev_user".into(),
            role: "developer".into(),
        };

        assert!(enforcer.allow_tool("shell", &admin).expect("policy evaluation failed"));
        assert!(!enforcer.allow_tool("shell", &developer).expect("policy evaluation failed"));
        assert!(enforcer
            .allow_tool("file_reader", &developer)
            .expect("policy evaluation failed"));
    }
}
