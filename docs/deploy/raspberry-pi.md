# 树莓派 / ARM64 部署

## 硬件要求

| 项目 | 最低要求 |
|------|---------|
| CPU | ARM64 (aarch64) |
| 内存 | 512MB 以上 |
| 系统 | Raspberry Pi OS / Armbian / Ubuntu Server |
| 存储 | 100MB 空闲 |

推荐：香橙派 Zero 3 (¥99) / 树莓派 4B (¥700)

## 一键部署

```bash
# 1. 下载 ARM64 二进制
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-arm64
mv mqtt-mcp-server-arm64 mqtt-mcp-server && chmod +x mqtt-mcp-server

# 2. 装 Mosquitto
sudo apt update && sudo apt install -y mosquitto

# 3. 下载部署脚本
wget https://raw.githubusercontent.com/baiyanlong/mqtt-mcp-server/main/deploy/install.sh
chmod +x install.sh

# 4. 一键安装
sudo ./install.sh
```

## 自定义参数

```bash
sudo ./install.sh \
  --broker tcp://192.168.1.100:1883 \
  --listen 0.0.0.0:3000 \
  --web-port 8080
```

## 部署后

```bash
# 服务状态
systemctl status mqtt-mcp-server

# 实时日志
journalctl -u mqtt-mcp-server -f

# 重启
systemctl restart mqtt-mcp-server
```

## 访问

```
SSE 端点:    http://树莓派IP:3000/sse
Web 面板:    http://树莓派IP:8080
API 端点:    http://树莓派IP:3000/message
```

## 连接 AI

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
