# MQTT MCP Cloud — Docker 镜像
#
# 构建： docker build -f cloud.Dockerfile -t mqtt-mcp-cloud:latest .
# 或配合 docker-compose： docker-compose -f cloud-docker-compose.yml build

FROM rust:1.86-slim-bookworm AS builder

WORKDIR /build
COPY . .

RUN apt-get update -qq && apt-get install -y -qq pkg-config libssl-dev && \
    cargo build --release --features cloud --bin mqtt-mcp-cloud

FROM debian:bookworm-slim

RUN apt-get update -qq && apt-get install -y -qq ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/mqtt-mcp-cloud /usr/local/bin/

EXPOSE 8080

ENTRYPOINT ["mqtt-mcp-cloud"]
CMD ["--listen", "0.0.0.0:8080"]
