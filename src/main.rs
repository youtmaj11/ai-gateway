mod agent;
mod cli;
mod config;
mod observability;
mod storage;
mod types;

use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, http::{Request, Response}, response::{IntoResponse, Json}, routing::get, serve, Router};
use clap::CommandFactory;
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, Span};

use cli::{Cli, Commands, ToolsCommand};
use storage::redis::RedisCache;

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

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let span = tracing::info_span!("ws.connection");
    let _enter = span.enter();
    info!("WebSocket connection opened");

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                info!(%text, "received websocket text");
                if socket.send(Message::Text(format!("echo: {text}"))).await.is_err() {
                    tracing::warn!("failed to send websocket response");
                    break;
                }
            }
            Message::Close(_) => {
                info!("websocket close received");
                break;
            }
            _ => {}
        }
    }

    info!("WebSocket connection closed");
}

#[tokio::main]
async fn main() {
    observability::init_tracing().expect("failed to initialize tracing");

    let cli = Cli::parse_args();
    let config = config::Config::from_file(&cli.config).expect("failed to load configuration");

    let mut cache = if !config.redis_url.is_empty() {
        match RedisCache::new(&config.redis_url).await {
            Ok(cache) => Some(cache),
            Err(err) => {
                tracing::warn!(%err, "failed to connect to redis, continuing without cache");
                None
            }
        }
    } else {
        None
    };

    if cli.server || cli.command.is_none() {
        let trace_layer = TraceLayer::new_for_http()
            .make_span_with(|request: &Request<_>| {
                tracing::info_span!("http.request",
                    method = %request.method(),
                    path = %request.uri().path(),
                )
            })
            .on_response(|response: &Response<_>, latency: Duration, span: &Span| {
                span.record("status_code", &tracing::field::display(response.status().as_u16()));
                info!(status = %response.status(), latency = ?latency, "request completed");
            });

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/ws", get(ws_handler))
            .layer(trace_layer);

        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

        info!(%addr, "starting ai-gateway HTTP server");
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
            let response = agent::core::run_chat(&message, cache.as_mut()).await;
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
