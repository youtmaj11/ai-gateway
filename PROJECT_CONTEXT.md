# PROJECT CONTEXT - Rust AI Agent Gateway

We are building a self-hosted, offline-first personal AI orchestrator (OpenClaw-style).

- Language: Rust 1.85+ only
- Web framework: Axum + Tower middleware
- Database: Postgres (SQLx type-safe)
- Cache/Rate limiting: Redis
- Queue: RabbitMQ (lapin or amqprs)
- Policy: OPA (Open Policy Agent)
- Auth: JWT
- Observability: OpenTelemetry + tracing
- Tools: 10 tools (shell, file reader with PDF support, PDF/book loader, web search via SearxNG, etc.)
- CLI first, WebSocket for real-time, REST API
- Local/offline mode is the default (daily use)
- GitOps deployment is optional for later (k8s folder exists as skeleton)

Use the exact directory structure provided above.
All code must be clean, well-commented, and production-ready.





Project Structure (exact):

ai-gateway/
├── Cargo.toml                          # Workspace + dependencies
├── src/
│   ├── main.rs                         # Axum server startup + WebSocket
│   ├── config.rs                       # Config loader (config.toml)
│   ├── cli.rs                          # CLI entrypoint using clap
│   ├── auth/                           # JWT validation middleware
│   ├── policy/                         # OPA client and policy loader
│   ├── agent/                          # Core agent loop and tool calling logic
│   ├── tools/                          # Individual tools (shell.rs, file_reader.rs, pdf_loader.rs, etc.)
│   ├── queue/                          # RabbitMQ producer and consumer
│   ├── storage/                        # SQLx Postgres + Redis client
│   ├── observability/                  # OpenTelemetry setup and tracing
│   └── types.rs                        # Shared structs and enums
├── migrations/                         # SQLx database migrations
├── k8s/apps/ai-gateway/                # GitOps manifests (create as skeleton now)
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── hpa.yaml
│   └── sealedsecret.yaml
├── config.toml.example
├── installer.sh                        # Idempotent local installer
├── README.md
├── architecture.md                     # Mermaid architecture diagram
├── Makefile
└── .gitignore