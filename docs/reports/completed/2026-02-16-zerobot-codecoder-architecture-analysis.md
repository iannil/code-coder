# ZeroBot 与 CodeCoder 架构分析完成报告

**完成时间**: 2026-02-16

## 分析目的

分析 ZeroBot (Rust) 和 CodeCoder (TypeScript) 两个项目的架构，识别重复实现和冗余逻辑。

## 分析结论

**两个项目不存在需要消除的重复实现或冗余逻辑。**

它们是互补设计，已有完整的双向集成机制。

## 项目定位

| 项目 | 语言 | 定位 | 特点 |
|------|------|------|------|
| ZeroBot | Rust | 轻量级 AI 助手基础设施/消息网关 | ~3.4MB, <5MB 内存, 24 个 LLM provider |
| CodeCoder | TypeScript | 专业 AI 编程工作台 | 23 个 Agent, TUI, LSP 集成 |

## 双向集成架构

```
┌─────────────────────────────────────────────────────────────────┐
│                     ZeroBot (Rust)                               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ CodeCoderTool → HTTP → CodeCoder API (:4096)            │    │
│  │ services/zero-bot/src/tools/codecoder.rs                │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              ↕ HTTP API
┌─────────────────────────────────────────────────────────────────┐
│                     CodeCoder (TypeScript)                       │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ ZeroBotMemoryProvider → SQLite → ZeroBot Memory         │    │
│  │ packages/ccode/src/memory-zerobot/provider.ts           │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 集成组件验证

### 1. ZeroBot → CodeCoder (HTTP API)

**文件**: `services/zero-bot/src/tools/codecoder.rs`

- ✅ 完整的 HTTP 客户端实现
- ✅ 支持所有 23 个 Agent
- ✅ Session 管理 (创建/复用)
- ✅ 消息轮询机制
- ✅ 超时处理 (2 分钟)
- ✅ 单元测试覆盖

### 2. CodeCoder Agent API

**文件**: `packages/ccode/src/api/server/handlers/agent.ts`

- ✅ `GET /api/agents` - 列出可用 Agent
- ✅ `POST /api/agent/invoke` - 调用 Agent
- ✅ `GET /api/agent/:agentId` - 获取 Agent 信息

**路由注册**: `packages/ccode/src/api/server/router.ts:187-190`

### 3. CodeCoder → ZeroBot (Memory 共享)

**文件**: `packages/ccode/src/memory-zerobot/provider.ts`

- ✅ SQLite 连接到 `~/.codecoder/workspace/memory/brain.db`
- ✅ `store()` - 存储记忆 (upsert)
- ✅ `recall()` - FTS5 全文搜索 + LIKE 回退
- ✅ `get()` - 按 key 获取
- ✅ `list()` - 按 category 列出
- ✅ `forget()` - 删除记忆
- ✅ `count()` - 统计数量
- ✅ 只读模式支持

## 重叠分析

### Provider 实现

| ZeroBot | CodeCoder | 结论 |
|---------|-----------|------|
| 24 个原生 Rust 实现 | Vercel AI SDK | 各自需要，非冗余 |

### Memory 系统

| ZeroBot | CodeCoder | 结论 |
|---------|-----------|------|
| `memory/sqlite.rs` | `memory-zerobot/` | 已集成共享 |
| `memory/markdown.rs` | `memory-markdown/` | 用途不同（日志 vs 长期） |

### Tool 实现

| Tool | ZeroBot | CodeCoder | 结论 |
|------|---------|-----------|------|
| Shell | `tools/shell.rs` | `tool/bash.ts` | 必要重复（独立运行） |
| File | `tools/file_*.rs` | `tool/read.ts` | 必要重复（独立运行） |

## 优化建议（未来考虑）

1. **内存双向同步**: 当前为单向读取，可考虑双向同步
2. **Tool Schema 标准化**: 提取共享的 JSON Schema 定义
3. **配置统一**: 统一 API key 等配置源

## 关键文件索引

| 组件 | 路径 |
|------|------|
| ZeroBot CodeCoder Tool | `services/zero-bot/src/tools/codecoder.rs` |
| CodeCoder Agent Handler | `packages/ccode/src/api/server/handlers/agent.ts` |
| CodeCoder Memory Provider | `packages/ccode/src/memory-zerobot/provider.ts` |
| Router 注册 | `packages/ccode/src/api/server/router.ts` |

## 验收结果

- ✅ 架构分析完成
- ✅ 无冗余需要消除
- ✅ 集成机制已验证
- ✅ 文档已更新
