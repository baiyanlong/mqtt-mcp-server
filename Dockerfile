# 多阶段构建 — 服务器端 Linux 原生编译
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY tests/ tests/
COPY config.example.yaml ./

RUN cargo build --release && strip target/release/mqtt-mcp-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mqtt-mcp-server /usr/local/bin/mqtt-mcp-server
COPY --from=builder /app/config.example.yaml /etc/mqtt-mcp/config.yaml

EXPOSE 8080

ENTRYPOINT ["mqtt-mcp-server"]
CMD ["--config", "/etc/mqtt-mcp/config.yaml", "--web", "8080"]
