# Provider 列表

ZeroBot 支持 24 个 LLM Provider。

## 一线 Provider

| Provider | 名称 | 环境变量 | 说明 |
|----------|------|----------|------|
| **Anthropic** | `anthropic` | `ANTHROPIC_API_KEY` | Claude 系列 |
| **OpenAI** | `openai` | `OPENAI_API_KEY` | GPT 系列 |
| **Google Gemini** | `gemini` | `GEMINI_API_KEY` | Gemini 系列 |
| **OpenRouter** | `openrouter` | `OPENROUTER_API_KEY` | 聚合路由 |

## 本地 Provider

| Provider | 名称 | 说明 |
|----------|------|------|
| **Ollama** | `ollama` | 本地模型，无需 API key |

## OpenAI 兼容 Provider

这些 Provider 使用 OpenAI 兼容 API:

| Provider | 名称 | 环境变量 | Base URL |
|----------|------|----------|----------|
| **Groq** | `groq` | `GROQ_API_KEY` | api.groq.com |
| **Mistral** | `mistral` | `MISTRAL_API_KEY` | api.mistral.ai |
| **xAI/Grok** | `xai` / `grok` | `XAI_API_KEY` | api.x.ai |
| **DeepSeek** | `deepseek` | `DEEPSEEK_API_KEY` | api.deepseek.com |
| **Together AI** | `together` | `TOGETHER_API_KEY` | api.together.xyz |
| **Fireworks AI** | `fireworks` | `FIREWORKS_API_KEY` | api.fireworks.ai |
| **Perplexity** | `perplexity` | `PERPLEXITY_API_KEY` | api.perplexity.ai |
| **Cohere** | `cohere` | `COHERE_API_KEY` | api.cohere.com |
| **Venice** | `venice` | - | api.venice.ai |
| **Vercel AI** | `vercel` | - | - |
| **Cloudflare AI** | `cloudflare` | - | - |
| **Moonshot/Kimi** | `moonshot` / `kimi` | - | api.moonshot.cn |
| **GLM/智谱** | `glm` / `zhipu` | - | open.bigmodel.cn |
| **MiniMax** | `minimax` | - | api.minimax.chat |
| **AWS Bedrock** | `bedrock` | - | - |
| **Qianfan/百度** | `qianfan` / `baidu` | - | aip.baidubce.com |
| **Synthetic** | `synthetic` | - | api.synthetic.com |
| **OpenCode** | `opencode` | - | - |
| **Z.AI** | `zai` / `z.ai` | - | - |

## 自定义 Provider

支持任意 OpenAI 兼容 API:

```toml
[provider]
name = "custom:https://my-llm.example.com"
api_key = "..."
```

格式: `custom:<base_url>`

---

## 配置示例

### Anthropic

```toml
[provider]
name = "anthropic"
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"
```

### OpenAI

```toml
[provider]
name = "openai"
api_key = "sk-..."
model = "gpt-4o"
```

### Gemini

```toml
[provider]
name = "gemini"
api_key = "..."
model = "gemini-2.0-flash"
```

### OpenRouter

```toml
[provider]
name = "openrouter"
api_key = "sk-or-..."
model = "anthropic/claude-3-5-sonnet"
```

### Ollama (本地)

```toml
[provider]
name = "ollama"
# 无需 api_key
model = "llama3.2"
```

### Groq

```toml
[provider]
name = "groq"
api_key = "gsk_..."
model = "llama-3.3-70b-versatile"
```

### DeepSeek

```toml
[provider]
name = "deepseek"
api_key = "..."
model = "deepseek-chat"
```

### 自定义

```toml
[provider]
name = "custom:https://my-llm.internal.company.com/v1"
api_key = "internal-key"
model = "internal-model"
```

---

## 弹性 Provider

支持自动故障转移:

```toml
[provider]
name = "openrouter"
api_key = "..."

[reliability]
fallback_chain = ["anthropic", "openai"]
retry_count = 3
retry_delay_ms = 1000
```

当主 provider 失败时，自动切换到 fallback 链中的下一个。

---

## API Key 来源

优先级（从高到低）:

1. 配置文件 `config.toml`
2. 环境变量
3. Provider 特定文件（如 `~/.gemini/oauth_creds.json`）

---

## 认证方式

| Provider | 认证方式 |
|----------|----------|
| 大多数 | Bearer Token |
| Gemini | API Key 或 OAuth |
| Ollama | 无认证 |
| Custom | Bearer Token |
