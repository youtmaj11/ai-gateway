mod agent;
mod auth;
mod cli;
mod config;
mod encryption;
mod middleware;
mod observability;
mod policy;
mod queue;
mod storage;
mod tools;
mod types;

use axum::{extract::{Extension, Query, ws::{Message, WebSocket, WebSocketUpgrade}}, http::{Request, Response}, response::{IntoResponse, Json}, routing::get, serve, Router};
use clap::CommandFactory;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, sync::mpsc};
use tower_http::trace::TraceLayer;
use tracing::{info, Span};

use agent::core::AgentCore;
use auth::jwt::JwtAuthLayer;
use cli::{Cli, Commands, ToolsCommand};
use middleware::rate_limit::RateLimitLayer;
use storage::redis::RedisCache;

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

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

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(config): Extension<config::Config>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, config))
}

async fn chat_handler(
    Query(params): Query<ChatRequest>,
    Extension(claims): Extension<auth::jwt::Claims>,
) -> Json<ChatResponse> {
    info!(%claims.sub, "authenticated chat request");

    Json(ChatResponse {
        response: format!("Authenticated agent response to: {}", params.message),
    })
}

async fn handle_socket(mut socket: WebSocket, config: config::Config) {
    let span = tracing::info_span!("ws.connection");
    let _enter = span.enter();
    info!("WebSocket connection opened");

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                info!(%text, "received websocket text");
                let (tx, mut rx) = mpsc::unbounded_channel();
                let prompt = text.clone();

                let agents = config.agents.clone();
                tokio::spawn(async move {
                    let _ = AgentCore::run_agent_stream(&prompt, tx, &agents).await;
                });

                while let Some(update) = rx.recv().await {
                    if socket.send(Message::Text(update)).await.is_err() {
                        tracing::warn!("failed to send websocket response");
                        break;
                    }
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

    storage::initialize_storage(&config)
        .await
        .expect("failed to initialize storage backend");

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

    let mut queue = if cli.server || cli.command.is_none() {
        None
    } else {
        Some(
            queue::create_queue(&config)
                .await
                .expect("failed to initialize queue backend"),
        )
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

        let rate_limit_layer = RateLimitLayer::new(&config.redis_url, config.rate_limit, config.rate_limit_window)
            .await
            .expect("failed to initialize rate limit middleware");

        let app = Router::new()
            .route(
                "/health",
                get(health_handler).layer::<RateLimitLayer, std::convert::Infallible>(rate_limit_layer.clone()),
            )
            .route(
                "/ws",
                get(ws_handler).layer::<RateLimitLayer, std::convert::Infallible>(rate_limit_layer.clone()),
            )
            .route(
                "/chat",
                get(chat_handler)
                    .layer::<RateLimitLayer, std::convert::Infallible>(rate_limit_layer.clone())
                    .layer(JwtAuthLayer::new(config.jwt_secret.clone())),
            )
            .layer(Extension(config.clone()))
            .layer(trace_layer);

        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

        info!(%addr, "starting ai-gateway HTTP server");
        let listener = TcpListener::bind(addr)
            .await
            .expect("failed to bind health server");

        let service = app.into_make_service_with_connect_info::<SocketAddr>();

        serve(listener, service)
            .await
            .expect("server failed");
        return;
    }

    match cli.command {
        Some(Commands::Chat { message }) => {
            let queue_ref = queue.as_ref().expect("queue backend was not initialized").as_ref();
            let response = agent::core::run_chat(&message, cache.as_mut(), queue_ref).await;
            println!("{response}");
        }
        Some(Commands::Tools { command: ToolsCommand::List }) => {
            println!("Available tools:");
            for tool in tools::registered_tools() {
                println!("- {tool}");
            }
        }
        Some(Commands::Tools { command: ToolsCommand::Run { tool, params, username, role } }) => {
            match tools::run_tool(&tool, &params, &username, &role).await {
                Ok(result) => println!("{result}"),
                Err(err) => eprintln!("Tool execution denied: {err}"),
            }
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
