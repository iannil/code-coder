# 核心 Traits

ZeroBot 使用 trait 驱动架构，每个子系统通过 trait 定义接口，实现可热切换。

## Trait 总览

| Trait | 文件 | 方法 | 用途 |
|-------|------|------|------|
| `Provider` | `providers/traits.rs` | `chat_with_system()` | LLM 聊天接口 |
| `Channel` | `channels/traits.rs` | `send()`, `listen()`, `health_check()` | 消息通道 |
| `Tool` | `tools/traits.rs` | `execute()`, `parameters_schema()` | Agent 能力 |
| `Memory` | `memory/traits.rs` | `store()`, `recall()`, `forget()` | 持久化存储 |
| `Observer` | `observability/traits.rs` | `record_event()`, `record_metric()` | 可观测性 |
| `RuntimeAdapter` | `runtime/traits.rs` | — | 平台适配 |
| `Tunnel` | `tunnel/mod.rs` | `start()`, `stop()`, `url()` | 隧道代理 |

---

## Provider Trait

LLM 提供者接口，支持 24 种后端。

```rust
// src/providers/traits.rs

#[async_trait]
pub trait Provider: Send + Sync {
    /// 使用系统提示和消息历史进行聊天
    async fn chat_with_system(
        &self,
        system: &str,
        messages: &[Message],
    ) -> anyhow::Result<String>;

    /// 返回提供者名称（用于日志/调试）
    fn name(&self) -> &str;
}

/// 聊天消息
pub struct Message {
    pub role: Role,      // User, Assistant, System
    pub content: String,
}
```

### 实现列表

| 实现 | 文件 | 说明 |
|------|------|------|
| `AnthropicProvider` | `anthropic.rs` | Claude 系列 |
| `OpenAiProvider` | `openai.rs` | GPT 系列 |
| `GeminiProvider` | `gemini.rs` | Gemini 系列 |
| `OllamaProvider` | `ollama.rs` | 本地模型 |
| `OpenRouterProvider` | `openrouter.rs` | 聚合路由 |
| `OpenAiCompatibleProvider` | `compatible.rs` | 20+ 兼容后端 |
| `ResilientProvider` | `reliable.rs` | 弹性包装 |

---

## Channel Trait

消息通道接口，支持 8 种通道。

```rust
// src/channels/traits.rs

#[async_trait]
pub trait Channel: Send + Sync {
    /// 发送消息到通道
    async fn send(&self, message: &str) -> anyhow::Result<()>;

    /// 监听传入消息（阻塞）
    async fn listen(&self) -> anyhow::Result<IncomingMessage>;

    /// 健康检查
    async fn health_check(&self) -> anyhow::Result<HealthStatus>;

    /// 通道名称
    fn name(&self) -> &str;
}

pub struct IncomingMessage {
    pub sender: String,
    pub content: String,
    pub channel: String,
    pub timestamp: DateTime<Utc>,
}

pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
```

### 实现列表

| 实现 | 文件 | 说明 |
|------|------|------|
| `CliChannel` | `cli.rs` | 终端交互 |
| `TelegramChannel` | `telegram.rs` | Telegram Bot |
| `DiscordChannel` | `discord.rs` | Discord Bot |
| `SlackChannel` | `slack.rs` | Slack App |
| `MatrixChannel` | `matrix.rs` | Matrix 协议 |
| `WhatsAppChannel` | `whatsapp.rs` | WhatsApp Business |
| `ImessageChannel` | `imessage.rs` | iMessage (macOS) |
| `EmailChannel` | `email_channel.rs` | IMAP/SMTP |

---

## Tool Trait

Agent 能力接口，支持 9 种工具。

```rust
// src/tools/traits.rs

#[async_trait]
pub trait Tool: Send + Sync {
    /// 执行工具
    async fn execute(&self, params: Value) -> anyhow::Result<ToolResult>;

    /// 返回 JSON Schema 描述参数
    fn parameters_schema(&self) -> Value;

    /// 工具名称
    fn name(&self) -> &str;

    /// 工具描述（供 LLM 理解）
    fn description(&self) -> &str;
}

pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
```

### 实现列表

| 实现 | 文件 | 说明 |
|------|------|------|
| `ShellTool` | `shell.rs` | Shell 命令 |
| `FileReadTool` | `file_read.rs` | 读取文件 |
| `FileWriteTool` | `file_write.rs` | 写入文件 |
| `MemoryStoreTool` | `memory_store.rs` | 存储记忆 |
| `MemoryRecallTool` | `memory_recall.rs` | 召回记忆 |
| `MemoryForgetTool` | `memory_forget.rs` | 遗忘记忆 |
| `BrowserTool` | `browser.rs` | 浏览器自动化 |
| `BrowserOpenTool` | `browser_open.rs` | 打开 URL |

---

## Memory Trait

持久化存储接口，支持 2 种后端。

```rust
// src/memory/traits.rs

#[async_trait]
pub trait Memory: Send + Sync {
    /// 存储记忆条目
    async fn store(&self, entry: MemoryEntry) -> anyhow::Result<String>;

    /// 召回相关记忆
    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>>;

    /// 遗忘指定记忆
    async fn forget(&self, id: &str) -> anyhow::Result<()>;

    /// 列出所有记忆
    async fn list(&self, category: Option<MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>>;
}

pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

pub enum MemoryCategory {
    Fact,        // 事实知识
    Preference,  // 用户偏好
    Context,     // 对话上下文
    Task,        // 任务信息
    Custom(String),
}
```

### 实现列表

| 实现 | 文件 | 说明 |
|------|------|------|
| `SqliteMemory` | `sqlite.rs` | SQLite + 向量搜索 |
| `MarkdownMemory` | `markdown.rs` | Markdown 文件 |

---

## Observer Trait

可观测性接口。

```rust
// src/observability/traits.rs

pub trait Observer: Send + Sync {
    /// 记录事件
    fn record_event(&self, event: &Event);

    /// 记录指标
    fn record_metric(&self, metric: &Metric);
}

pub struct Event {
    pub name: String,
    pub level: Level,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

pub struct Metric {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
}
```

### 实现列表

| 实现 | 文件 | 说明 |
|------|------|------|
| `LogObserver` | `log.rs` | 日志输出 |
| `MultiObserver` | `multi.rs` | 多 Observer 组合 |
| `NoopObserver` | `noop.rs` | 空实现 |

---

## 添加新实现

1. 在对应模块创建新文件 `src/<subsystem>/your_impl.rs`
2. 实现对应的 trait
3. 在 `src/<subsystem>/mod.rs` 的工厂函数中注册

示例：添加新 Provider

```rust
// src/providers/my_provider.rs
pub struct MyProvider { /* ... */ }

#[async_trait]
impl Provider for MyProvider {
    async fn chat_with_system(&self, system: &str, messages: &[Message]) -> Result<String> {
        // 实现
    }

    fn name(&self) -> &str { "my-provider" }
}

// src/providers/mod.rs
pub fn create_provider(name: &str, api_key: Option<&str>) -> Result<Box<dyn Provider>> {
    match name {
        // ... 现有匹配
        "my-provider" => Ok(Box::new(my_provider::MyProvider::new(api_key))),
        // ...
    }
}
```
