# ZeroBot 文档

> 轻量级、基于 trait 的 AI 助手基础设施

## 快速导航

| 分类 | 文档 | 说明 |
|------|------|------|
| **架构** | [OVERVIEW.md](architecture/OVERVIEW.md) | 系统架构概览 |
| | [TRAITS.md](architecture/TRAITS.md) | 核心 Trait 接口详解 |
| | [DATA_FLOW.md](architecture/DATA_FLOW.md) | 数据流与请求处理 |
| **指南** | [QUICK_START.md](guides/QUICK_START.md) | 快速入门 |
| | [CONFIGURATION.md](guides/CONFIGURATION.md) | 配置详解 |
| | [EXTENDING.md](guides/EXTENDING.md) | 扩展指南 |
| **参考** | [CLI.md](reference/CLI.md) | CLI 命令参考 |
| | [PROVIDERS.md](reference/PROVIDERS.md) | Provider 列表 |
| | [CHANNELS.md](reference/CHANNELS.md) | Channel 列表 |
| **开发** | [PROGRESS.md](development/PROGRESS.md) | 项目进展 |
| | [CHANGELOG.md](development/CHANGELOG.md) | 变更日志 |
| | [CONTRIBUTING.md](development/CONTRIBUTING.md) | 贡献指南 |
| | [TESTING.md](development/TESTING.md) | 测试指南 |
| **归档** | [RENAME_HISTORY.md](archive/RENAME_HISTORY.md) | 重命名历史 |

## 项目概览

```
ZeroBot v0.1.0
├── 24 个 LLM Providers (Anthropic, OpenAI, Gemini, OpenRouter, ...)
├── 8 个 Channels (CLI, Telegram, Discord, Slack, Matrix, WhatsApp, iMessage, Email)
├── 2 个 Memory 后端 (SQLite+向量, Markdown)
├── 8 个 Tools (shell, file_read/write, memory_*, browser, codecoder)
├── 5 个 Tunnel 类型 (Cloudflare, ngrok, Tailscale, Custom, None)
├── 1,811+ 测试
├── ~3.4MB 二进制
└── <5MB 内存占用
```

## 核心设计原则

1. **Trait 驱动** — 每个子系统都是一个 trait，通过配置切换实现
2. **安全默认** — 沙箱化一切，使用白名单
3. **最小依赖** — 每个 crate 都会增加二进制大小
4. **生产就绪** — 禁止 unwrap，完善错误处理

## 相关文件

- [CLAUDE.md](../CLAUDE.md) — Claude Code 工作指南
- [Cargo.toml](../Cargo.toml) — 项目依赖
