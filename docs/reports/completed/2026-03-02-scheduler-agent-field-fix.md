# 修复：定时任务 Agent 调用失败 - "agent is required" 错误

**日期:** 2026-03-02
**状态:** 已完成
**Trace ID:** 36f639bb-ea4a-4faf-b2ba-92ce0b978781

## 问题描述

用户通过 Telegram 创建的会议提醒定时任务无法执行，手动触发时返回错误：
```
agent is required
```

## 根本原因分析

### 问题链路

1. `scheduler.ts` 的 `runSchedulerTask` 函数在执行 agent 类型任务时，调用 `/api/agent/invoke` API
2. 请求 body 使用了错误的字段名 `agentId`
3. `agent.ts` 的 `InvokeAgentRequest` 接口期望字段名是 `agent`
4. 字段名不匹配导致 `input.agent` 为 `undefined`，触发 400 错误

### 代码定位

**错误代码** (`packages/ccode/src/api/server/handlers/scheduler.ts:598`):
```typescript
body: JSON.stringify({
  agentId: command.agentName,  // ❌ 错误的字段名
  prompt: command.prompt,
})
```

**API 期望** (`packages/ccode/src/api/server/handlers/agent.ts:27-29`):
```typescript
interface InvokeAgentRequest {
  agent: string   // ✓ 正确的字段名
  prompt: string
}
```

## 修复方案

修改 `scheduler.ts` 中的字段名：

```typescript
body: JSON.stringify({
  agent: command.agentName,  // ✓ 使用正确的字段名
  prompt: command.prompt,
})
```

## 同时修复的问题

### Trace 查询系统跨服务支持

发现 `trace show` 命令无法查询 Rust 服务的 trace（存储在 `zero-*.log` 文件中）。

**修复内容** (`packages/ccode/src/trace/query.ts`):
1. 新增 `getServiceLogFiles()` - 查找 `zero-*.log` 文件
2. 新增 `parseServiceLogFile()` - 从混合格式日志中提取 JSON 条目
3. 新增 `normalizeLogEntry()` - 处理 `timestamp` → `ts` 字段映射（Rust 与 TypeScript 格式差异）
4. 更新 `queryTrace()` - 同时搜索 trace JSONL 和服务日志文件

## 验证

- [x] TypeScript 类型检查通过
- [x] Trace 查询系统可以正确显示 Rust 服务的 trace 条目

## 影响范围

- 定时任务的 agent 类型任务现在可以正常执行
- `trace show` 命令现在支持跨服务 trace 关联

## 后续建议

1. 为 scheduler API 添加单元测试，验证请求格式
2. 考虑在 API 层添加 Zod schema 验证，提供更友好的错误提示
3. 统一 Rust 和 TypeScript 的 trace 格式（推荐统一使用 `ts` 字段）
