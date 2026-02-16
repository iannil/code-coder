# 快速入门

## 安装

### 从源码编译

```bash
git clone <repo-url>
cd zero-bot
cargo install --path .
```

### 验证安装

```bash
zero-bot --version
```

## 首次设置

### 快速设置（推荐）

```bash
zero-bot onboard
```

这会:
1. 创建配置目录 `~/.codecoder/`
2. 检测可用的 API key
3. 生成基础 `config.toml`

### 交互式向导

```bash
zero-bot onboard --interactive
```

完整引导配置所有选项。

## 基本使用

### 单条消息

```bash
zero-bot agent -m "你好，请介绍一下自己"
```

### 交互式聊天

```bash
zero-bot agent
```

### 系统状态

```bash
zero-bot status
```

### 诊断

```bash
zero-bot doctor
```

## 配置

配置文件位于 `~/.codecoder/config.toml`。

### 最小配置

```toml
[provider]
name = "openrouter"
api_key = "sk-..."
model = "anthropic/claude-3-5-sonnet"

[memory]
backend = "sqlite"
```

### 完整配置示例

```toml
# Provider 配置
[provider]
name = "anthropic"
api_key = "sk-ant-..."
model = "claude-3-5-sonnet"

# Memory 后端
[memory]
backend = "sqlite"
vector_weight = 0.7
keyword_weight = 0.3

# Gateway 配置 (可选)
[gateway]
enabled = false
port = 8080

# Tunnel 配置 (可选)
[tunnel]
kind = "cloudflare"

# Identity
[identity]
name = "ZeroBot"
```

## 下一步

- [CONFIGURATION.md](CONFIGURATION.md) — 完整配置说明
- [../reference/CLI.md](../reference/CLI.md) — CLI 命令参考
- [../reference/PROVIDERS.md](../reference/PROVIDERS.md) — Provider 列表
