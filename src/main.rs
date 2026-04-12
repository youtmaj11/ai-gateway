mod config;
mod cli;
mod types;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config = config::Config::from_env().expect("failed to load configuration");
    let app = Router::<()>::new().route("/", get(|| async { "ai-gateway is running" }));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(%addr, "starting ai-gateway server");
    let _ = app;
    tracing::info!(%addr, "server stub initialized");
}
