mod agent;
mod cli;
mod config;
mod types;

use axum::{response::Json, routing::get, serve, Router};
use clap::CommandFactory;
use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cli::{Cli, Commands, ToolsCommand};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse_args();
    let config = config::Config::from_file(&cli.config).expect("failed to load configuration");

    if cli.server || cli.command.is_none() {
        let app = Router::new().route("/health", get(health_handler));
        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

        tracing::info!(%addr, "starting ai-gateway HTTP server");
        let listener = TcpListener::bind(addr)
            .await
            .expect("failed to bind health server");

        serve(listener, app)
            .await
            .expect("server failed");
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
