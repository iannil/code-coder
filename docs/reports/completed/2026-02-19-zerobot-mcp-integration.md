# ZeroBot MCP 集成实现报告

**日期**: 2026-02-19
**状态**: 已完成

## 概述

实现了 ZeroBot 的 MCP (Model Context Protocol) 双向支持，让 ZeroBot 既能作为 MCP Client 连接外部 MCP 服务器，也能作为 MCP Server 暴露内部工具。

## 实现内容

### Phase 1: MCP 协议基础层

| 文件 | 说明 | 行数 |
|------|------|------|
| `src/mcp/types.rs` | JSON-RPC 2.0 + MCP 协议类型定义 | ~350 |
| `src/mcp/transport.rs` | Stdio/HTTP 传输层实现 | ~200 |

### Phase 2: MCP Client + Server

| 文件 | 说明 | 行数 |
|------|------|------|
| `src/mcp/client.rs` | MCP Client，连接外部 MCP 服务器 | ~200 |
| `src/mcp/server.rs` | MCP Server，暴露 ZeroBot 工具 | ~280 |
| `src/mcp/adapter.rs` | MCP Tool ↔ ZeroBot Tool 适配器 | ~130 |

### Phase 3: 集成

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/mcp/mod.rs` | 新增 | MCP 模块入口，McpManager |
| `src/tools/registry.rs` | 新增 | 统一工具注册中心 |
| `src/config/schema.rs` | 修改 | 添加 McpConfig, McpServerConfig |
| `src/config/mod.rs` | 修改 | 导出 MCP 配置类型 |
| `src/main.rs` | 修改 | 添加 mcp-server CLI 命令 |
| `src/lib.rs` | 修改 | 导出 mcp 模块 |
| `src/gateway/mod.rs` | 修改 | 添加 /mcp 路由 |
| `src/tools/mod.rs` | 修改 | 导出 ToolRegistry |

## 配置格式

`~/.codecoder/config.json` 中的 MCP 配置：

```json
{
  "mcp": {
    "servers": {
      "filesystem": {
        "type": "local",
        "command": ["npx", "-y", "@anthropic/mcp-filesystem", "/path"],
        "enabled": true
      },
      "remote-api": {
        "type": "remote",
        "url": "https://api.example.com/mcp",
        "headers": { "Authorization": "Bearer xxx" },
        "enabled": true
      }
    },
    "server_enabled": true,
    "server_api_key": "optional-api-key"
  }
}
```

## CLI 命令

```bash
# 作为 MCP Server 运行（stdio 模式，供其他客户端调用）
zero-bot mcp-server --stdio

# 作为 MCP Server 运行（HTTP 模式）
zero-bot mcp-server
```

## Gateway 集成

当 `mcp.server_enabled = true` 时，Gateway 会在 `/mcp` 路径暴露 MCP JSON-RPC 端点。

## 测试结果

```
running 33 tests
test mcp::adapter::tests::adapter_prefixed_name ... ok
test mcp::adapter::tests::extract_text_content_empty ... ok
test mcp::adapter::tests::extract_text_content_mixed ... ok
test mcp::adapter::tests::extract_text_content_multiple ... ok
test mcp::adapter::tests::extract_text_content_single ... ok
test mcp::adapter::tests::mcp_tool_schema_preserved ... ok
test mcp::client::tests::client_call_tool_fails_when_not_initialized ... ok
test mcp::client::tests::client_not_initialized_by_default ... ok
test mcp::client::tests::client_server_name ... ok
test mcp::server::tests::server_handles_initialize ... ok
test mcp::server::tests::server_handles_tool_not_found ... ok
test mcp::server::tests::server_handles_tools_call ... ok
test mcp::server::tests::server_handles_tools_list ... ok
test mcp::server::tests::server_handles_unknown_method ... ok
test mcp::server::tests::server_mcp_tools_conversion ... ok
test mcp::tests::mcp_manager_default ... ok
test mcp::tests::mcp_manager_get_client_not_found ... ok
test mcp::tests::mcp_manager_new ... ok
test mcp::transport::tests::http_transport_new ... ok
test mcp::transport::tests::http_transport_with_headers ... ok
test mcp::transport::tests::stdio_transport_empty_command_fails ... ok
test mcp::transport::tests::stdio_transport_invalid_command_fails ... ok
test mcp::types::tests::call_tool_result_serialization ... ok
test mcp::types::tests::initialize_params_deserialization ... ok
test mcp::types::tests::json_rpc_id_from_number ... ok
test mcp::types::tests::json_rpc_id_from_string ... ok
test mcp::types::tests::json_rpc_request_serialization ... ok
test mcp::types::tests::json_rpc_response_error ... ok
test mcp::types::tests::json_rpc_response_success ... ok
test mcp::types::tests::mcp_tool_serialization ... ok
test mcp::types::tests::notification_has_no_id ... ok
test mcp::types::tests::tool_content_text ... ok
test tools::registry::tests::registry_mcp_tool_count_empty ... ok

test result: ok. 33 passed; 0 failed; 0 ignored
```

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                         ZeroBot                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────┐     ┌─────────────────┐               │
│  │   MCP Client    │     │   MCP Server    │               │
│  │  (连接外部 MCP)  │     │ (暴露内部 Tools) │               │
│  └────────┬────────┘     └────────┬────────┘               │
│           │                       │                         │
│           ▼                       ▼                         │
│  ┌─────────────────────────────────────────┐               │
│  │           Tool Registry (统一层)          │               │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐    │               │
│  │  │ Native  │ │  MCP    │ │CodeCoder│    │               │
│  │  │ Tools   │ │ Tools   │ │ Tools   │    │               │
│  │  └─────────┘ └─────────┘ └─────────┘    │               │
│  └─────────────────────────────────────────┘               │
│                       │                                     │
│                       ▼                                     │
│              AgentExecutor (LLM 调用)                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 设计决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 依赖选择 | 自实现 JSON-RPC | 更可控，无外部依赖风险 |
| 实现优先级 | 同时实现 Client + Server | 架构更完整 |
| 认证方式 | API Key | 简单实用，满足需求 |

## 后续工作

1. 集成测试：配置真实 MCP 服务器测试端到端流程
2. Agent 集成：在 AgentExecutor 中使用 ToolRegistry
3. 文档完善：添加用户使用指南

## 总计

- **新增代码**: ~1600 行
- **新增文件**: 7 个
- **修改文件**: 6 个
- **测试用例**: 33 个
