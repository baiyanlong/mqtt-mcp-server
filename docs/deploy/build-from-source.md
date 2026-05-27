# 从源码编译

## 前提

- Rust 1.75+
- Git

## 编译

```bash
git clone https://github.com/baiyanlong/mqtt-mcp-server.git
cd mqtt-mcp-server
cargo build --release
# 产出: target/release/mqtt-mcp-server
```

## ARM64 交叉编译

在 x86_64 机器上交叉编译给树莓派：

```bash
# 安装交叉编译器
sudo apt install -y gcc-aarch64-linux-gnu

# 添加目标 + 编译
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# 产出: target/aarch64-unknown-linux-gnu/release/mqtt-mcp-server
```

或在 macOS 上用 zig 交叉编译：

```bash
brew install zig
cargo install cargo-zigbuild
rustup target add aarch64-unknown-linux-gnu
cargo zigbuild --release --target aarch64-unknown-linux-gnu
```

## Cloud 服务 (Pro)

```bash
cargo build --release --features cloud --bin mqtt-mcp-cloud
# 产出: target/release/mqtt-mcp-cloud
```

## 运行测试

```bash
cargo test      # 23 个测试
```
