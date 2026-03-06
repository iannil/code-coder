# Wave 2.5: Core NAPI Extension - 完成报告

> **状态**: ✅ 已完成
> **日期**: 2026-03-06
> **依赖**: Wave 2 验证完成 (eff4c67)

## 摘要

Wave 2.5 扩展了 Core NAPI 绑定，添加了 MCP prompts/resources 和 LSP workspace_symbol/call hierarchy 功能，为 Wave 2.6 的 ccode 重构奠定基础。

## 完成的工作

### 1. MCP Prompts/Resources (Rust)

**文件**: `services/zero-core/src/protocol/mcp_client.rs`

- 添加 `list_prompts()` 方法
- 添加 `get_prompt(name, arguments)` 方法
- 已有的 `list_resources()` 和 `read_resource()` 方法

**文件**: `services/zero-core/src/protocol/mcp.rs`

新增类型:
- `McpPrompt`
- `McpPromptArgument`
- `McpPromptResult`
- `McpPromptMessage`
- `McpPromptContent` (enum)
- `McpPromptTextContent`
- `McpPromptImageContent`
- `McpPromptResourceContent`
- `McpEmbeddedResource`

### 2. MCP Prompts/Resources (NAPI)

**文件**: `services/zero-core/src/napi/protocol.rs`

- 添加 `list_resources()` NAPI 方法
- 添加 `read_resource()` NAPI 方法
- 添加 `list_prompts()` NAPI 方法
- 添加 `get_prompt()` NAPI 方法

### 3. LSP Workspace/Call Hierarchy (Rust)

**文件**: `services/zero-core/src/protocol/lsp.rs`

新增方法:
- `workspace_symbol(key, query)` - 搜索工作区符号
- `prepare_call_hierarchy(key, uri, line, character)` - 准备调用层次项
- `incoming_calls(key, item)` - 获取传入调用
- `outgoing_calls(key, item)` - 获取传出调用

新增类型:
- `LspRange`
- `LspWorkspaceSymbol`
- `LspCallHierarchyItem`
- `LspCallHierarchyIncomingCall`
- `LspCallHierarchyOutgoingCall`

### 4. LSP Workspace/Call Hierarchy (NAPI)

**文件**: `services/zero-core/src/napi/protocol.rs`

- 添加 `workspace_symbol()` NAPI 方法
- 添加 `prepare_call_hierarchy()` NAPI 方法
- 添加 `incoming_calls()` NAPI 方法
- 添加 `outgoing_calls()` NAPI 方法

### 5. TypeScript Wrappers

**文件**: `packages/core/src/protocol.ts`

- 添加 MCP prompts/resources 接口
- 添加 LSP workspace symbol 和 call hierarchy 接口
- 更新 `IMcpClientManager` 接口
- 更新 `ILspServerManager` 接口

**文件**: `packages/core/src/mcp.ts`

- 添加 `listResources()` 方法
- 添加 `readResource()` 方法
- 添加 `listPrompts()` 方法
- 添加 `getPrompt()` 方法

**文件**: `packages/core/src/lsp.ts`

- 添加 `workspaceSymbol()` 方法
- 添加 `prepareCallHierarchy()` 方法
- 添加 `incomingCalls()` 方法
- 添加 `outgoingCalls()` 方法

**文件**: `packages/core/src/binding.d.ts`

- 添加所有新的 TypeScript 类型声明

## 验证结果

```bash
# Rust 编译
cargo check  # ✅ 通过 (5 warnings)

# TypeScript 类型检查
bun run typecheck  # ✅ 通过

# 测试
bun test  # ✅ 159 pass, 6 skip, 0 fail
```

## 下一步 (Wave 2.6)

Wave 2.6 将使用这些新的 Core 绑定重构 ccode 模块:

1. **MCP 重构** (`packages/ccode/src/mcp/index.ts`)
   - 当前: 948 行，使用 `@modelcontextprotocol/sdk`
   - 目标: ~100-150 行，使用 `@codecoder-ai/core`

2. **LSP 重构** (`packages/ccode/src/lsp/index.ts` + `server.ts`)
   - 当前: ~2,500 行
   - 目标: ~150-200 行，使用 `@codecoder-ai/core`

## 架构说明

```
┌─────────────────────────────────────────────────────────────┐
│                     packages/ccode                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ MCP Module (948 行) → 重构后 ~100 行                 │   │
│  │ LSP Module (2500 行) → 重构后 ~150 行                │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           packages/core (TypeScript)                │   │
│  │ McpClientManager, LspServerManager                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │        services/zero-core (Rust + NAPI)             │   │
│  │ McpClientManagerHandle, LspServerManagerHandle      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

`★ Insight ─────────────────────────────────────`
**设计决策**:
1. **NAPI 层只做桥接** - 不添加业务逻辑，保持薄层设计
2. **TypeScript wrapper 处理类型转换** - snake_case → camelCase
3. **ccode 层保留高级特性** - OAuth 浏览器流程、通知处理等
`─────────────────────────────────────────────────`
