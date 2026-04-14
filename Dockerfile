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

RUN groupadd --system --gid 10001 app && \
    useradd --system --uid 10001 --gid app --home-dir /nonexistent --shell /usr/sbin/nologin app

WORKDIR /usr/local/bin

COPY --from=builder --chown=app:app /usr/src/ai-gateway/target/release/ai-gateway ./ai-gateway

USER app

HEALTHCHECK --interval=30s --timeout=5s CMD ["sh", "-c", "grep -q ai-gateway /proc/1/cmdline"]

EXPOSE 8080

CMD ["./ai-gateway"]
