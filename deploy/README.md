# ARM64 部署包

树莓派 / ARM64 Linux 设备一键部署。

## 文件

```
deploy/
├── install.sh                  # 一键安装脚本（systemd 服务）
└── README.md                   # 本文件
```

## 使用方法

### 1. 获取 ARM 二进制

从 GitHub Releases 下载 `mqtt-mcp-server-arm64`，或自行交叉编译：

```bash
# 在 x86_64 Linux 服务器上（需安装 zig）
cargo install cargo-zigbuild
rustup target add aarch64-unknown-linux-gnu
cargo zigbuild --release --target aarch64-unknown-linux-gnu
# 产出: target/aarch64-unknown-linux-gnu/release/mqtt-mcp-server
```

### 2. 部署到树莓派

```bash
# 把二进制、安装脚本、config.yaml（可选）放到设备上
scp mqtt-mcp-server-arm64 install.sh config.yaml pi@树莓派IP:~/

# SSH 进去执行
ssh pi@树莓派IP
sudo ./install.sh

# 自定义参数
sudo ./install.sh \
  --broker tcp://192.168.1.100:1883 \
  --listen 0.0.0.0:3000 \
  --web-port 8080
```

### 3. AI Agent 连接

安装完成后，用 Claude Desktop / Cursor / MCP Inspector 连接：

```
SSE 端点: http://树莓派IP:3000/sse
Web 面板: http://树莓派IP:8080
```

Claude Desktop 配置：

```json
{
  "mcpServers": {
    "mqtt": {
      "transport": "sse",
      "url": "http://树莓派IP:3000/sse"
    }
  }
}
```

## 常用命令

```bash
systemctl status mqtt-mcp-server     # 查看状态
journalctl -u mqtt-mcp-server -f     # 实时日志
systemctl restart mqtt-mcp-server    # 重启
systemctl stop mqtt-mcp-server       # 停止
```

## 设备要求

- ARM64 CPU（树莓派 3B+/4/5、CM4 等）
- Raspberry Pi OS (Debian Bookworm) 或 Ubuntu Server
- Mosquitto MQTT Broker: `sudo apt install mosquitto`
- systemd
