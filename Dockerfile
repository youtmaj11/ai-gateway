# Build stage
FROM rust:1.72-slim as builder

WORKDIR /usr/src/ai-gateway

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 1000 app && \
    useradd --system --uid 1000 --gid app --home-dir /nonexistent --shell /usr/sbin/nologin app

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/ai-gateway/target/release/ai-gateway ./ai-gateway
RUN chown app:app ./ai-gateway

USER app

EXPOSE 8080

CMD ["./ai-gateway"]
