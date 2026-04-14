use std::fmt;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::runtime::Builder;
use tokio::time::timeout;

#[derive(Debug)]
pub enum ShellExecutorError {
    DisallowedCommand(String),
    Io(std::io::Error),
    Timeout,
    Execution(String),
}

impl fmt::Display for ShellExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellExecutorError::DisallowedCommand(cmd) => {
                write!(f, "disallowed command: {cmd}")
            }
            ShellExecutorError::Io(err) => write!(f, "I/O error: {err}"),
            ShellExecutorError::Timeout => write!(f, "command timed out"),
            ShellExecutorError::Execution(err) => write!(f, "execution error: {err}"),
        }
    }
}

impl std::error::Error for ShellExecutorError {}

impl From<std::io::Error> for ShellExecutorError {
    fn from(error: std::io::Error) -> Self {
        ShellExecutorError::Io(error)
    }
}

pub struct ShellExecutorTool;

impl ShellExecutorTool {
    const ALLOWED_COMMANDS: [&'static str; 6] = ["echo", "pwd", "date", "whoami", "uname", "uptime"];

    fn is_safe_arg(arg: &str) -> bool {
        !arg.contains('/')
            && !arg.contains('\'')
            && !arg.contains('|')
            && !arg.contains('&')
            && !arg.contains(';')
            && !arg.contains('>')
            && !arg.contains('<')
            && !arg.contains('$')
            && !arg.contains('`')
            && !arg.contains('\n')
            && !arg.contains('\r')
    }

    fn parse_params(params: &str) -> Result<(String, Vec<String>), ShellExecutorError> {
        let mut parts = params.split_whitespace();
        let command = parts
            .next()
            .ok_or_else(|| ShellExecutorError::Execution("missing command".to_string()))?
            .to_string();
        let args: Vec<String> = parts.map(String::from).collect();

        if !Self::ALLOWED_COMMANDS.contains(&command.as_str()) {
            return Err(ShellExecutorError::DisallowedCommand(command));
        }

        if !command.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(ShellExecutorError::DisallowedCommand(command));
        }

        if args.iter().any(|arg| !Self::is_safe_arg(arg)) {
            return Err(ShellExecutorError::Execution("disallowed argument detected".to_string()));
        }

        Ok((command, args))
    }

    async fn execute_command(command: String, args: Vec<String>) -> Result<String, ShellExecutorError> {
        let mut cmd = Command::new(command);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .current_dir("/tmp");

        let output = timeout(Duration::from_secs(10), cmd.output())
            .await
            .map_err(|_| ShellExecutorError::Timeout)??;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr))
    }
}

impl ShellExecutorTool {
    pub async fn execute_async(params: &str) -> String {
        let result = match Self::parse_params(params) {
            Ok((cmd, args)) => Self::execute_command(cmd, args).await,
            Err(err) => Err(err),
        };

        match result {
            Ok(output) => output,
            Err(err) => format!("Error executing shell command: {err}"),
        }
    }
}

impl crate::tools::Tool for ShellExecutorTool {
    fn name(&self) -> &'static str {
        "shell_executor"
    }

    fn execute(&self, params: &str) -> String {
        format!("ShellExecutorTool should be invoked through async run_tool; params={params}")
    }
}
