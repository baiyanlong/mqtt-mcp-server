# Docker 部署

## 边缘节点

```bash
# 从源码构建镜像
docker build -t mqtt-mcp-server .

# 运行
docker run -d \
  --name mqtt-mcp \
  -p 3000:3000 -p 8080:8080 \
  mqtt-mcp-server \
  --mode sse --listen 0.0.0.0:3000 \
  --broker tcp://host.docker.internal:1883 \
  --topics '#'
```

## Docker Compose (含 Mosquitto)

```yaml
version: "3.8"
services:
  mosquitto:
    image: eclipse-mosquitto:2
    ports: ["1883:1883"]

  mqtt-mcp:
    build: .
    ports: ["3000:3000", "8080:8080"]
    command:
      - --mode=sse
      - --listen=0.0.0.0:3000
      - --broker=tcp://mosquitto:1883
      - --topics=#
    depends_on: [mosquitto]
```

```bash
docker-compose up -d
```

## Cloud 云服务 (Pro)

```bash
# PostgreSQL + Cloud 一键启动
docker-compose -f cloud-docker-compose.yml up -d

# 访问
open http://localhost:8080
```
