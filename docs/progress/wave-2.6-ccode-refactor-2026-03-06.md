# Wave 2.6: ccode 重构 - 进展报告

> **状态**: ✅ 已完成 (部分)
> **日期**: 2026-03-06
> **依赖**: Wave 2.5 完成

## 摘要

Wave 2.6 使用 `@codecoder-ai/core` 绑定重构 ccode 模块，减少 TypeScript 代码量。

## 完成的工作

### 1. MCP 重构 ✅

**文件**: `packages/ccode/src/mcp/index.ts`

- **之前**: 948 行，使用 `@modelcontextprotocol/sdk`
- **之后**: 443 行，使用 `@codecoder-ai/core`
- **减少**: 53% (505 行)

**变更内容**:
- 替换 `@modelcontextprotocol/sdk` 为 `McpClientManager` from Core
- 删除 `McpOAuthProvider` 类 (Core 内部处理)
- 保留 ccode 特定功能:
  - `Instance.state()` 生命周期管理
  - AI SDK `dynamicTool` 转换
  - Bus 事件发布 (ToolsChanged, BrowserOpenFailed)
  - TUI toast 集成
  - 浏览器 OAuth 流程 (McpOAuthCallback)
  - 类型定义 (Status, Resource, PromptMessage 等)

**辅助修改**:
- `packages/ccode/src/api/server/handlers/mcp.ts`: 更新 `finishAuth` API 签名 (添加 state 参数)

**验证**:
```bash
bun run typecheck  # ✅ 通过
```

### 2. LSP 重构 ⏭️ 跳过

**分析结论**: LSP 重构不切实际，原因如下:

**server.ts 复杂性** (2,046 行):
- 35 个服务器定义，每个都有 TypeScript 特定的 spawn 逻辑
- 使用 `Bun.resolve`, `bunx`, `Bun.spawn` 等 Bun 特定 API
- 自动下载缺失服务器的逻辑
- 每个服务器的自定义 initialization 选项

**迁移风险**:
- 需要将所有 spawn 逻辑移植到 Rust
- 可能破坏现有功能
- 收益不明显 (ccode LSP 已经工作正常)

**决定**: 保持现有 LSP 实现不变，它工作稳定且功能完整。

## 最终架构

```
┌─────────────────────────────────────────────────────────────┐
│                     packages/ccode                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ MCP Module (948→443 行) ✅ 使用 Core                │   │
│  │ LSP Module (2783 行) ⏭️ 保持不变                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           packages/core (TypeScript)                │   │
│  │ McpClientManager ✅, LspServerManager (未使用)      │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │        services/zero-core (Rust + NAPI)             │   │
│  │ McpClientManagerHandle ✅, LspServerManagerHandle   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 总结

| 模块 | 之前 | 之后 | 减少 | 状态 |
|------|------|------|------|------|
| MCP | 948 行 | 443 行 | 53% | ✅ 完成 |
| LSP | 2,783 行 | 2,783 行 | 0% | ⏭️ 跳过 |
| **总计** | 3,731 行 | 3,226 行 | **14%** | - |

`★ Insight ─────────────────────────────────────`
**教训**:
1. MCP 重构成功因为 Core 完全处理 MCP 协议
2. LSP 重构困难因为 ccode 有大量 TypeScript 特定的服务器 spawn 逻辑
3. 未来若要重构 LSP，需要先将服务器定义移植到 Rust
`─────────────────────────────────────────────────`
