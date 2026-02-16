# 项目进展

> 最后更新: 2026-02

## 版本信息

- **版本**: 0.1.0
- **Rust Edition**: 2021
- **测试数量**: 1,811+
- **二进制大小**: ~3.4MB
- **内存占用**: <5MB

---

## 模块成熟度

| 模块 | 状态 | 测试覆盖 | 说明 |
|------|------|----------|------|
| **providers** | ✅ 稳定 | 高 | 24 个后端，完善测试 |
| **channels** | ✅ 稳定 | 高 | 8 个通道，已验证 |
| **tools** | ✅ 稳定 | 高 | 9 个工具，沙箱保护 |
| **memory** | ✅ 稳定 | 高 | SQLite+向量，基准测试 |
| **security** | ✅ 稳定 | 高 | 加密存储，沙箱策略 |
| **config** | ✅ 稳定 | 高 | TOML schema |
| **gateway** | ✅ 稳定 | 中 | Axum webhook |
| **daemon** | ✅ 稳定 | 中 | 自主运行时 |
| **tunnel** | ✅ 稳定 | 高 | 5 种隧道 |
| **onboard** | ✅ 稳定 | 中 | 设置向导 |
| **skills** | ⚠️ Beta | 中 | SKILL.md 加载 |
| **observability** | ✅ 稳定 | 高 | 日志/指标 |
| **cron** | ⚠️ Beta | 中 | 定时调度 |
| **integrations** | ⚠️ Beta | 低 | 外部集成 |

### 状态说明

- ✅ **稳定** — 生产就绪，API 稳定
- ⚠️ **Beta** — 功能完整，API 可能变化
- 🚧 **开发中** — 正在实现
- ❌ **计划中** — 尚未开始

---

## 已实现功能

### Providers (24 个)

| 类别 | Provider | 状态 |
|------|----------|------|
| **一线** | Anthropic (Claude) | ✅ |
| | OpenAI (GPT) | ✅ |
| | Google (Gemini) | ✅ |
| | OpenRouter | ✅ |
| **本地** | Ollama | ✅ |
| **兼容层** | Venice | ✅ |
| | Vercel AI | ✅ |
| | Cloudflare AI | ✅ |
| | Moonshot/Kimi | ✅ |
| | Groq | ✅ |
| | Mistral | ✅ |
| | xAI/Grok | ✅ |
| | DeepSeek | ✅ |
| | Together AI | ✅ |
| | Fireworks AI | ✅ |
| | Perplexity | ✅ |
| | Cohere | ✅ |
| | GLM/智谱 | ✅ |
| | MiniMax | ✅ |
| | AWS Bedrock | ✅ |
| | Qianfan/百度 | ✅ |
| | Synthetic | ✅ |
| | OpenCode | ✅ |
| | Z.AI | ✅ |
| | Custom (任意 URL) | ✅ |

### Channels (8 个)

| Channel | 状态 | 说明 |
|---------|------|------|
| CLI | ✅ | 终端交互 |
| Telegram | ✅ | Bot API |
| Discord | ✅ | Bot + Gateway |
| Slack | ✅ | App + Events |
| Matrix | ✅ | Synapse 兼容 |
| WhatsApp | ✅ | Business API |
| iMessage | ✅ | macOS 限定 |
| Email | ✅ | IMAP/SMTP |

### Tools (9 个)

| Tool | 状态 | 说明 |
|------|------|------|
| shell | ✅ | 沙箱命令执行 |
| file_read | ✅ | 安全读取 |
| file_write | ✅ | 安全写入 |
| memory_store | ✅ | 存储记忆 |
| memory_recall | ✅ | 向量召回 |
| memory_forget | ✅ | 选择性遗忘 |
| browser | ✅ | Playwright 自动化 |
| browser_open | ✅ | 打开 URL |
| composio | ✅ | 外部 API 集成 |

### Memory (2 个)

| Backend | 状态 | 特性 |
|---------|------|------|
| SQLite | ✅ | 向量嵌入、FTS5、混合搜索 |
| Markdown | ✅ | 人类可读、Git 友好 |

### Tunnel (5 个)

| Tunnel | 状态 |
|--------|------|
| Cloudflare | ✅ |
| ngrok | ✅ |
| Tailscale | ✅ |
| Custom | ✅ |
| None | ✅ |

---

## 待实现功能

### 高优先级

- [ ] **Streaming 响应** — Provider trait 支持流式输出
- [ ] **多模态** — 图片/音频/视频输入
- [ ] **Function Calling** — 标准化工具调用协议
- [ ] **Web UI** — 基础管理界面

### 中优先级

- [ ] **插件系统** — 动态加载 Rust/WASM 插件
- [ ] **多用户** — 用户隔离和权限
- [ ] **Rate Limiting** — 细粒度限流
- [ ] **Prometheus 导出** — 指标监控

### 低优先级

- [ ] **语音通道** — 语音输入/输出
- [ ] **视频通道** — 视频通话集成
- [ ] **分布式** — 多节点部署

---

## 技术债务

| 项目 | 优先级 | 说明 |
|------|--------|------|
| 统一错误类型 | 中 | 部分模块使用 `anyhow`，部分用 `thiserror` |
| 日志规范化 | 低 | 统一日志格式和级别 |
| 文档补全 | 中 | 部分公共 API 缺少文档注释 |
| 基准测试 | 低 | 添加更多性能基准 |

---

## 代码统计

```
源文件:    78 个 .rs 文件
测试:      1,811 个测试
模块:      20 个子模块
LOC:       ~15,000 行 (估计)
依赖:      ~50 个 crate
```

---

## 里程碑

| 版本 | 目标 | 状态 |
|------|------|------|
| 0.1.0 | 核心功能完整 | ✅ 完成 |
| 0.2.0 | Streaming + 多模态 | 🚧 进行中 |
| 0.3.0 | Web UI + 插件 | ❌ 计划中 |
| 1.0.0 | 生产稳定版 | ❌ 计划中 |
