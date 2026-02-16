# 架构概览

## 系统架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                           ZeroBot 架构                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐          │
│  │   Channels   │    │    Agent     │    │   Providers  │          │
│  │              │◄───┤              ├───►│              │          │
│  │ CLI/Telegram │    │  循环编排     │    │ 24 LLM 后端  │          │
│  │ Discord/...  │    └──────┬───────┘    └──────────────┘          │
│  └──────────────┘           │                                       │
│                             │                                       │
│  ┌──────────────┐    ┌──────▼───────┐    ┌──────────────┐          │
│  │    Memory    │◄───┤    Tools     │───►│   Security   │          │
│  │              │    │              │    │              │          │
│  │ SQLite/MD    │    │ 9 能力工具    │    │ 沙箱/策略     │          │
│  └──────────────┘    └──────────────┘    └──────────────┘          │
│                                                                      │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐          │
│  │   Gateway    │    │    Daemon    │    │    Tunnel    │          │
│  │              │    │              │    │              │          │
│  │ HTTP/Webhook │    │  自主运行时   │    │ CF/ngrok/... │          │
│  └──────────────┘    └──────────────┘    └──────────────┘          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## 模块职责

| 模块 | 路径 | 职责 | 核心 Trait |
|------|------|------|------------|
| **providers** | `src/providers/` | LLM 后端接口 | `Provider` |
| **channels** | `src/channels/` | 消息通道 | `Channel` |
| **tools** | `src/tools/` | Agent 能力 | `Tool` |
| **memory** | `src/memory/` | 持久化存储 | `Memory` |
| **observability** | `src/observability/` | 指标/日志 | `Observer` |
| **runtime** | `src/runtime/` | 平台适配 | `RuntimeAdapter` |
| **tunnel** | `src/tunnel/` | 隧道代理 | `Tunnel` |
| **security** | `src/security/` | 沙箱安全 | `SecurityPolicy` |
| **agent** | `src/agent/` | 循环编排 | — |
| **gateway** | `src/gateway/` | HTTP webhook | — |
| **daemon** | `src/daemon/` | 自主运行时 | — |
| **config** | `src/config/` | TOML 配置 | — |
| **onboard** | `src/onboard/` | 设置向导 | — |
| **skills** | `src/skills/` | TOML+SKILL.md | — |

## 目录结构

```
src/                         # 78 个源文件
├── lib.rs                   # 库入口，全局 lint 配置
├── main.rs                  # CLI 入口，命令路由
├── providers/               # LLM Providers
│   ├── mod.rs              # 工厂函数 create_provider()
│   ├── traits.rs           # Provider trait 定义
│   ├── anthropic.rs        # Anthropic Claude
│   ├── openai.rs           # OpenAI GPT
│   ├── gemini.rs           # Google Gemini
│   ├── ollama.rs           # 本地 Ollama
│   ├── openrouter.rs       # OpenRouter 聚合
│   ├── compatible.rs       # OpenAI 兼容层 (20+ 后端)
│   └── reliable.rs         # 弹性包装器 (重试/备选)
├── channels/                # 消息通道
│   ├── mod.rs              # 通道注册
│   ├── traits.rs           # Channel trait 定义
│   ├── cli.rs              # 命令行交互
│   ├── telegram.rs         # Telegram Bot
│   ├── discord.rs          # Discord Bot
│   ├── slack.rs            # Slack App
│   ├── matrix.rs           # Matrix Protocol
│   ├── whatsapp.rs         # WhatsApp Business
│   ├── imessage.rs         # iMessage (macOS)
│   └── email_channel.rs    # Email (IMAP/SMTP)
├── tools/                   # Agent 工具
│   ├── mod.rs              # 工具注册
│   ├── traits.rs           # Tool trait 定义
│   ├── shell.rs            # Shell 命令执行
│   ├── file_read.rs        # 文件读取
│   ├── file_write.rs       # 文件写入
│   ├── memory_store.rs     # 存储记忆
│   ├── memory_recall.rs    # 召回记忆
│   ├── memory_forget.rs    # 遗忘记忆
│   ├── browser.rs          # 浏览器自动化
│   ├── browser_open.rs     # 打开 URL
│   └── composio.rs         # Composio 集成
├── memory/                  # 记忆后端
│   ├── mod.rs              # Memory 工厂
│   ├── traits.rs           # Memory trait 定义
│   ├── sqlite.rs           # SQLite + 向量搜索
│   ├── markdown.rs         # Markdown 文件
│   ├── embeddings.rs       # 嵌入向量提供者
│   ├── vector.rs           # 向量相似度
│   ├── chunker.rs          # 文本分块
│   └── hygiene.rs          # 记忆清理
├── security/                # 安全子系统
│   ├── mod.rs              # 安全模块入口
│   ├── policy.rs           # SecurityPolicy 实现
│   ├── secrets.rs          # 加密密钥存储
│   └── pairing.rs          # 设备配对
├── tunnel/                  # 隧道代理
│   ├── mod.rs              # Tunnel 工厂
│   ├── cloudflare.rs       # Cloudflare Tunnel
│   ├── ngrok.rs            # ngrok
│   ├── tailscale.rs        # Tailscale Funnel
│   ├── custom.rs           # 自定义命令
│   └── none.rs             # 无隧道 (localhost)
├── observability/           # 可观测性
│   ├── mod.rs              # Observer 工厂
│   ├── traits.rs           # Observer trait 定义
│   ├── log.rs              # 日志 Observer
│   ├── multi.rs            # 多 Observer 组合
│   └── noop.rs             # 空 Observer
├── runtime/                 # 运行时适配
│   ├── mod.rs              # Runtime 工厂
│   ├── traits.rs           # RuntimeAdapter trait
│   └── native.rs           # 原生运行时
├── agent/                   # Agent 核心
│   ├── mod.rs              # Agent 入口
│   └── loop_.rs            # 主循环逻辑
├── gateway/                 # HTTP 网关
│   └── mod.rs              # Axum webhook 服务器
├── daemon/                  # 守护进程
│   └── mod.rs              # 自主运行时
├── config/                  # 配置
│   ├── mod.rs              # 配置加载
│   └── schema.rs           # TOML schema
├── onboard/                 # 设置向导
│   ├── mod.rs              # 快速设置
│   └── wizard.rs           # 交互式向导
├── skills/                  # 技能加载
│   ├── mod.rs              # TOML manifest 解析
│   └── symlink_tests.rs    # 符号链接测试
├── integrations/            # 外部集成
│   ├── mod.rs              # 集成入口
│   └── registry.rs         # 集成注册表
├── cron/                    # 定时任务
│   ├── mod.rs              # Cron 入口
│   └── scheduler.rs        # 调度器
├── heartbeat/               # 心跳
│   ├── mod.rs              # 心跳入口
│   └── engine.rs           # 心跳引擎
├── health/                  # 健康检查
│   └── mod.rs              # 健康状态
├── doctor/                  # 诊断
│   └── mod.rs              # 系统诊断
├── service/                 # 服务管理
│   └── mod.rs              # systemd/launchd
├── migration.rs             # 数据迁移
└── util.rs                  # 通用工具
```

## 设计原则

### 1. Trait 驱动架构

每个子系统定义一个 trait，实现可通过配置切换：

```rust
// 定义 trait
pub trait Provider: Send + Sync {
    fn chat_with_system(&self, system: &str, messages: &[Message]) -> Result<String>;
}

// 工厂函数根据配置创建实现
pub fn create_provider(name: &str, api_key: Option<&str>) -> Result<Box<dyn Provider>>
```

### 2. 安全默认

- 沙箱化所有工具执行
- 白名单而非黑名单
- 加密存储敏感信息

### 3. 最小化依赖

- 每个 crate 增加二进制大小
- 优先使用标准库
- 谨慎添加新依赖

### 4. 错误处理

- 禁止 `unwrap()` / `expect()`
- 使用 `anyhow::Result` 或 `thiserror`
- 提供有意义的错误信息

## 数据流

详见 [DATA_FLOW.md](DATA_FLOW.md)
