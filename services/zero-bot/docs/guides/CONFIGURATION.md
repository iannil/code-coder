# 配置详解

配置文件: `~/.codecoder/config.toml`

## 配置结构

```toml
[provider]     # LLM Provider 配置
[memory]       # Memory 后端配置
[gateway]      # HTTP Gateway 配置
[tunnel]       # 隧道配置
[autonomy]     # 自主模式配置
[identity]     # Bot 身份配置
[runtime]      # 运行时配置
```

---

## [provider]

LLM Provider 配置。

```toml
[provider]
name = "anthropic"          # provider 名称
api_key = "sk-..."          # API key (可选，支持加密)
model = "claude-3-5-sonnet" # 模型名称
```

### 支持的 Provider

| name | 说明 |
|------|------|
| `anthropic` | Anthropic Claude |
| `openai` | OpenAI GPT |
| `gemini` | Google Gemini |
| `openrouter` | OpenRouter 聚合 |
| `ollama` | 本地 Ollama |
| `groq` | Groq |
| `mistral` | Mistral |
| `deepseek` | DeepSeek |
| ... | 详见 [PROVIDERS.md](../reference/PROVIDERS.md) |

### 自定义 Provider

```toml
[provider]
name = "custom:https://my-llm.example.com"
api_key = "..."
```

---

## [memory]

Memory 后端配置。

```toml
[memory]
backend = "sqlite"       # sqlite 或 markdown
vector_weight = 0.7      # 向量搜索权重
keyword_weight = 0.3     # 关键词搜索权重
cache_max = 10000        # 嵌入缓存大小
```

### SQLite 后端

```toml
[memory]
backend = "sqlite"
# 数据库位于 ~/.codecoder/memory/brain.db
```

特性:
- 向量嵌入存储
- FTS5 全文搜索
- 混合搜索

### Markdown 后端

```toml
[memory]
backend = "markdown"
# 记忆存储在 ~/.codecoder/memory/*.md
```

特性:
- 人类可读
- Git 友好
- 易于编辑

---

## [gateway]

HTTP Gateway 配置。

```toml
[gateway]
enabled = true
host = "127.0.0.1"
port = 8080
```

### Webhook 端点

启用后可接收 webhook:

```
POST /webhook/telegram
POST /webhook/slack
POST /webhook/discord
...
```

---

## [tunnel]

隧道配置，用于公网访问。

```toml
[tunnel]
kind = "cloudflare"     # 隧道类型
```

### 支持的隧道类型

| kind | 说明 |
|------|------|
| `cloudflare` | Cloudflare Tunnel |
| `ngrok` | ngrok |
| `tailscale` | Tailscale Funnel |
| `custom` | 自定义命令 |
| `none` | 无隧道 (仅本地) |

### Cloudflare Tunnel

```toml
[tunnel]
kind = "cloudflare"
# 需要 cloudflared 已登录
```

### ngrok

```toml
[tunnel]
kind = "ngrok"
# 需要 NGROK_AUTHTOKEN 环境变量
```

### 自定义隧道

```toml
[tunnel]
kind = "custom"
command = "my-tunnel start"
```

---

## [autonomy]

自主模式配置 (daemon)。

```toml
[autonomy]
enabled = true
check_interval_secs = 60    # 检查间隔
```

---

## [identity]

Bot 身份配置。

```toml
[identity]
name = "ZeroBot"
description = "AI 助手"
```

---

## [runtime]

运行时配置。

```toml
[runtime]
kind = "native"     # 目前仅支持 native
```

---

## 环境变量

API key 可通过环境变量设置:

| 环境变量 | Provider |
|----------|----------|
| `ANTHROPIC_API_KEY` | anthropic |
| `OPENAI_API_KEY` | openai |
| `OPENROUTER_API_KEY` | openrouter |
| `GEMINI_API_KEY` | gemini |
| `GROQ_API_KEY` | groq |
| ... | ... |

## 加密存储

API key 可加密存储:

```toml
[provider]
api_key = "enc2:..."  # 加密值
```

使用 `zero-bot onboard` 自动加密。

---

## 完整示例

```toml
# ~/.codecoder/config.toml

[provider]
name = "openrouter"
api_key = "enc2:..."
model = "anthropic/claude-3-5-sonnet"

[memory]
backend = "sqlite"
vector_weight = 0.7
keyword_weight = 0.3

[gateway]
enabled = true
port = 8080

[tunnel]
kind = "cloudflare"

[autonomy]
enabled = false

[identity]
name = "MyBot"

[runtime]
kind = "native"
```
