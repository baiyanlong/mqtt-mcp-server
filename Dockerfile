# MQTT MCP Server 边缘节点 — 多架构 Docker 镜像
#
# 构建（自动检测架构）：
#   docker build -t mqtt-mcp-server .
#
# 构建 ARM64（在 x86 上交叉）：
#   docker buildx build --platform linux/arm64 -t mqtt-mcp-server .
#
# 运行：
#   docker run -d --name mqtt-mcp \
#     -p 3000:3000 -p 8080:8080 \
#     -e MQTT_BROKER=tcp://mosquitto:1883 \
#     mqtt-mcp-server

FROM rust:1.86-slim-bookworm AS builder

WORKDIR /build
COPY . .

RUN apt-get update -qq && apt-get install -y -qq pkg-config libssl-dev && \
    cargo build --release

FROM debian:bookworm-slim

RUN apt-get update -qq && apt-get install -y -qq ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/mqtt-mcp-server /usr/local/bin/

# 默认配置
ENV MQTT_BROKER=tcp://localhost:1883
ENV MQTT_TOPICS="#"

EXPOSE 3000 8080

ENTRYPOINT ["mqtt-mcp-server"]
CMD ["--mode", "sse", \
     "--listen", "0.0.0.0:3000", \
     "--web", "8080"]
