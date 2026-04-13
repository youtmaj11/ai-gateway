# Build stage
FROM rust:1.72-slim as builder

WORKDIR /usr/src/ai-gateway

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/ai-gateway/target/release/ai-gateway ./ai-gateway

EXPOSE 8080

CMD ["./ai-gateway"]
