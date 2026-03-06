# Phase 6.1: Session Loop 核心 - LLM 调用迁移

**日期**: 2026-03-05
**状态**: 完成

## 概述

将 LLM 调用从 TypeScript 迁移到 Rust，工具执行通过 IPC 回调到 TypeScript。

## 完成内容

### 1. IPC 协议扩展

**Rust 侧** (`services/zero-cli/src/ipc/protocol.rs`):
- 新增 `AgentPromptParams` - Agent 提示请求参数
- 新增 `AgentPromptResult` - 返回 request_id 用于流关联
- 新增 `AgentStreamEvent` - 流事件枚举 (TextDelta, ToolCall, Finish 等)
- 新增 `AgentStreamNotification` - 流事件通知
- 新增 `TokenUsage` - Token 使用统计
- 新增 `ModelInfo` - 模型信息

**TypeScript 侧** (`packages/ccode/src/ipc/types.ts`):
- 镜像所有 Rust 类型定义
- 添加 `agent_stream` 到 `IpcEvents`

### 2. 流式 LLM Provider 实现

**新增文件**: `services/zero-agent/src/streaming.rs`
- `StreamingProvider` trait - 流式 Provider 接口
- `AnthropicProvider` - Claude API 实现
- `StreamEvent` - 流事件类型
- `Message`, `Role`, `ContentPart` - 消息类型
- `ToolDef` - 工具定义
- SSE 解析器 for Anthropic API

### 3. AgentLoopHandler 实现

**修改文件**: `services/zero-cli/src/ipc/server.rs`
- `handle_agent_prompt` - 处理 agent 提示请求
- 支持取消 (`handle_cancel_generation`)
- 工具结果回调 (`handle_tool_result`)
- 流事件通过 IPC 通知发送

### 4. TypeScript IPC 客户端更新

**修改文件**: `packages/ccode/src/ipc/client.ts`
- `agentPrompt()` - 异步生成器方法
- `setToolExecutor()` - 设置工具执行回调
- 工具调用时自动执行并发送结果

### 5. TUI 后端集成

**修改文件**: `packages/ccode/src/cli/cmd/tui/backend/ipc.ts`
- 添加 `agent_stream` 事件处理
- `mapAgentStreamEvent()` - 转换为 TUI 事件
- `handleAgentPrompt()` - RPC 方法处理

## 架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                     TUI / CLI (TypeScript)                          │
│                 packages/ccode/src/cli/cmd/tui/                     │
└──────────────────────────────┬──────────────────────────────────────┘
                               │ IPC (AgentPrompt request)
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     zero-cli IPC Server (Rust)                      │
│                 services/zero-cli/src/ipc/server.rs                 │
│                                                                     │
│   AgentLoopHandler:                                                 │
│   1. 接收 prompt 请求                                                │
│   2. 调用 LLM (via StreamingProvider)                               │
│   3. 解析工具调用                                                    │
│   4. 发送 ToolCall 通知 → TS 执行 → 返回 ToolResult                  │
│   5. 流式返回文本响应                                                │
└─────────────────────────────────────────────────────────────────────┘
```

## 文件变更

| 文件 | 操作 | 描述 |
|------|------|------|
| `services/zero-agent/Cargo.toml` | 修改 | 添加 reqwest, futures-util 依赖 |
| `services/zero-agent/src/lib.rs` | 修改 | 导出 streaming 模块 |
| `services/zero-agent/src/streaming.rs` | 新增 | 流式 Provider 实现 |
| `services/zero-cli/src/ipc/protocol.rs` | 修改 | 添加 AgentPrompt 协议类型 |
| `services/zero-cli/src/ipc/server.rs` | 修改 | 添加 AgentLoopHandler |
| `packages/ccode/src/ipc/types.ts` | 修改 | 添加 AgentPrompt 类型定义 |
| `packages/ccode/src/ipc/protocol.ts` | 修改 | 处理 agent_stream 通知 |
| `packages/ccode/src/ipc/client.ts` | 修改 | 添加 agentPrompt 方法 |
| `packages/ccode/src/ipc/index.ts` | 修改 | 导出新类型 |
| `packages/ccode/src/cli/cmd/tui/backend/ipc.ts` | 修改 | 集成 agent loop |

## 测试结果

```
Rust:
- zero-agent: 5 tests passed (streaming 模块)
- zero-cli: 13 tests passed (protocol 模块)
- 总计: 520 tests passed, 0 failed

TypeScript:
- tsc --noEmit: 通过，无错误
```

## 下一步

- Phase 6.2: 将高频工具 (Read, Edit, Grep, Glob) 完全在 Rust 执行
- Phase 6.3: 完整循环迁移，TypeScript 只负责 UI

## 风险和缓解

| 风险 | 缓解策略 |
|------|----------|
| LLM 流式响应在 IPC 中延迟 | 使用 Unix Domain Socket，低延迟 |
| 工具执行失败 | 错误通过 ToolResult 返回，不阻塞循环 |
| API Key 管理 | 支持配置文件和参数覆盖 |
