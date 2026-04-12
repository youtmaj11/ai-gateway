mod agent;
mod cli;
mod config;
mod types;

use axum::{routing::get, Router};
use clap::CommandFactory;
use std::net::SocketAddr;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cli::{Cli, Commands, ToolsCommand};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse_args();

    if cli.server {
        let config = config::Config::from_file(&cli.config).expect("failed to load configuration");
        let app = Router::<()>::new().route("/", get(|| async { "ai-gateway is running" }));

        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
        tracing::info!(%addr, "starting ai-gateway server");
        let _ = app;
        tracing::info!(%addr, "server stub initialized");
        return;
    }

    match cli.command {
        Some(Commands::Chat { message }) => {
            let response = agent::core::run_chat(&message);
            println!("{response}");
        }
        Some(Commands::Tools { command: ToolsCommand::List }) => {
            println!("Available tools:");
            println!("- shell");
            println!("- file_reader");
            println!("- pdf_loader");
            println!("- book_loader");
        }
        Some(Commands::Version) => {
            println!("ai-gateway {}", env!("CARGO_PKG_VERSION"));
        }
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().expect("failed to render help");
            println!();
        }
    }
}
