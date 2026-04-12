use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ai-gateway")]
pub struct Cli {
    /// Configuration file path
    #[arg(long, default_value = "config.toml")]
    pub config: String,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
