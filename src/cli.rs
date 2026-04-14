use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "ai-gateway")]
#[command(about = "Self-hosted offline-first AI orchestrator gateway")]
pub struct Cli {
    /// Configuration file path
    #[arg(long, default_value = "config.toml")]
    pub config: String,

    /// Run the server instead of the CLI
    #[arg(long)]
    pub server: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Send a chat message to the AI agent
    Chat {
        /// Message to send
        message: String,
    },
    /// Tool management commands
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },
    /// Print version information
    Version,
}

#[derive(Subcommand, Debug)]
pub enum ToolsCommand {
    /// List available tools
    List,
    /// Run a tool with policy authorization
    Run {
        /// Name of the tool to execute
        tool: String,
        /// Parameters passed to the tool (for example a file path)
        #[arg(long, default_value = "")]
        params: String,
        /// User role used for authorization checks
        #[arg(long, default_value = "developer")]
        role: String,
        /// Username used for tool tracking
        #[arg(long, default_value = "cli_user")]
        username: String,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}

pub struct CliProgress;

impl CliProgress {
    pub fn spinner(message: impl Into<String>) -> ProgressBar {
        let bar = ProgressBar::new_spinner();
        bar.set_message(message.into());
        bar.set_style(
            ProgressStyle::with_template("{spinner:.green} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        bar.enable_steady_tick(Duration::from_millis(80));
        bar
    }

    pub fn done(bar: ProgressBar, message: impl Into<String>) {
        bar.finish_with_message(message.into());
    }
}
