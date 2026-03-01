# IM 渠道工具结果不返回问题修复

**日期**: 2026-02-28
**状态**: 已完成

## 问题描述

用户报告通过 IM (Telegram/Discord等) 调用工具时，工具结果没有完整返回到 IM。

### 用户表现

用户只看到：
- "🚀 开始处理..."
- "⚡ 执行工具 websearch"
- 然后就没有后续了（或只看到"✅ 处理完成"）

### 影响的工具

- WebSearch / WebFetch（搜索类工具）
- Read / Write / Edit（文件操作）
- Bash（命令执行）
- Grep / Glob（代码搜索）
- 其他所有工具

## 修复方案

### 方案 1: 在 Rust 端累积**所有**工具结果

**文件**: `services/zero-channels/src/progress.rs`

修改 `on_tool_use` 方法，将所有工具的结果累积到 tracker 中：

```rust
// 累积工具结果到 tracker（适用于所有工具）
if let Some(ref result) = event.result {
    let tracker_ref = self.get_tracker(&msg.id);
    let mut tracker = tracker_ref.lock().await;

    let result_text = self.format_tool_result_for_accumulation(&event.tool, result);
    if !result_text.is_empty() {
        if tracker.current_text == "[started]" {
            tracker.current_text = result_text;
        } else {
            tracker.current_text.push_str("\n\n");
            tracker.current_text.push_str(&result_text);
        }
    }
}
```

对于大型输出（如文件内容、命令输出），结果会被截断到 2000 字符以避免超过消息大小限制。

### 方案 2: 在 TypeScript 端包含**所有**工具结果

**文件**: `packages/ccode/src/api/server/handlers/task.ts`

在生成 output 时，如果有工具调用结果但没 text parts，将**所有**工具结果包含在输出中：

```typescript
// 如果没有 text parts，尝试从工具结果生成输出
else if (toolResultParts.length > 0) {
  const formattedToolResults = toolResultParts
    .map((part) => {
      const invocation = (part as { toolInvocation: { toolName: string; result: string } })
        .toolInvocation
      return formatToolResult(invocation.toolName, invocation.result)
    })
    .filter(Boolean)
    .join("\n\n")

  output = formattedToolResults || "✅ 处理完成"
}
```

对于大型输出，结果会被截断到 2000 字符并显示原始大小。

### 方案 3: 在工具通知中显示结果预览

**文件**: `services/zero-channels/src/progress.rs`

对于搜索类工具，在工具通知消息中显示完整结果；对于其他工具，显示截断的预览。

## 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `services/zero-channels/src/progress.rs` | 方案 1 和 3：累积和显示工具结果 |
| `packages/ccode/src/api/server/handlers/task.ts` | 方案 2：确保输出包含工具结果 |

## 新增/修改辅助函数

### Rust (`progress.rs`)

- `is_search_tool(tool: &str) -> bool`: 判断是否为搜索类工具
- `format_search_result(&self, tool: &str, result: &serde_json::Value) -> String`: 格式化搜索结果用于即时通知
- `format_tool_result_for_accumulation(&self, tool: &str, result: &serde_json::Value) -> String`: **格式化所有工具结果**用于累积到最终输出，大型输出会被截断到 2000 字符
- `format_search_result_for_accumulation(&self, tool: &str, result: &serde_json::Value) -> String`: 格式化搜索结果用于累积

### TypeScript (`task.ts`)

- `formatToolResult(toolName: string, result: string): string | null`: **格式化所有工具结果**用于任务输出，大型输出会被截断到 2000 字符
- `formatToolDisplayName(toolName: string): string`: 获取工具的显示名称（带 emoji）

## 验证步骤

1. 通过 Telegram 发送搜索请求
2. 验证 IM 收到完整的搜索结果
3. 检查日志确认工具结果被正确处理
4. 测试各种情况：
   - 简单搜索
   - 复杂搜索（大量结果）
   - 搜索失败（网络错误等）

## 实施时间线

- 2026-02-28: 初始实施（仅搜索类工具）
- 2026-02-28: 扩展到**所有工具**，支持 Read/Bash/Grep 等所有工具的结果累积

## 注意事项

- 大型输出会被截断到 2000 字符以避免超过 Telegram 消息大小限制（4096 字符）
- 截断时会显示原始输出大小，如 `[输出已截断，共 5432 字符]`
- 如果 AI 生成了文本输出，工具结果不会被累积（避免重复）
- 工具结果以代码块格式显示，便于阅读

## 后续修复 (2026-02-28 14:00+)

### 问题：AI 执行工具后不继续分析

用户报告 Telegram 只显示工具执行结果，但 AI 没有继续生成分析内容。

#### 根因分析

1. **finishReason 校正只对 Gemini 生效**
   - `processor.ts` 中的 finishReason 校正原本只针对 Gemini 模型
   - 当其他模型（如 Zhipu AI glm-5）在工具调用后发送 `end-turn` 而非 `tool-calls` 时，循环提前退出

2. **合成用户消息只对 Gemini 生效**
   - `prompt.ts` 中在工具调用后添加合成用户消息以提示 AI 继续
   - 该逻辑原本只针对 Gemini 模型

3. **JSON 解析错误 (truncateArgs bug)**
   - `processor.ts` 中的 `truncateArgs` 函数尝试解析截断后的 JSON
   - 截断后的 JSON 是无效的（如 `{"key": "val...`），导致 "Unterminated string" 错误

#### 修复方案

##### 修复 1: finishReason 校正扩展到所有模型

**文件**: `packages/ccode/src/session/processor.ts`

```typescript
// Fix finishReason: if there are completed tool calls but finishReason is not "tool-calls",
// force correct it to ensure the loop continues properly.
// This applies to ALL models (Gemini, Claude, etc.) as some models may send "end-turn"
// after tool calls when they should continue processing the tool results.
const hasCompletedToolCalls =
  Object.keys(toolcalls).length > 0 ||
  (await MessageV2.parts(input.assistantMessage.id)).some(
    (p) => p.type === "tool" && (p.state.status === "completed" || p.state.status === "error"),
  )

if (hasCompletedToolCalls && value.finishReason !== "tool-calls") {
  input.assistantMessage.finish = "tool-calls"
} else {
  input.assistantMessage.finish = value.finishReason
}
```

##### 修复 2: 合成用户消息扩展到所有模型

**文件**: `packages/ccode/src/session/prompt.ts`

```typescript
// Some models need synthetic user message after tool calls to prompt continuation.
// This was originally for Gemini, but other models (like zhipu-ai/glm-5) also benefit.
// Adding this for ALL models when result is "continue" and there are completed tool calls.
if (result === "continue") {
  const lastParts = await MessageV2.parts(processor.message.id)
  const hasToolCalls = lastParts.some(
    (p) => p.type === "tool" && (p.state.status === "completed" || p.state.status === "error"),
  )
  if (hasToolCalls) {
    // Add synthetic user message to prompt AI to continue...
  }
}
```

##### 修复 3: 修复 truncateArgs JSON 解析错误

**文件**: `packages/ccode/src/session/processor.ts`

```typescript
// Helper: truncate tool args for display (returns string for large args)
const truncateArgs = (args: unknown): unknown => {
  if (args === null || args === undefined) return args
  const str = JSON.stringify(args)
  if (str.length > 200) {
    // Return truncated string directly - don't try to parse invalid JSON
    return str.slice(0, 200) + "..."
  }
  return args
}
```

#### 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `packages/ccode/src/session/processor.ts` | 修复 1 和 3：finishReason 校正 + truncateArgs 修复 |
| `packages/ccode/src/session/prompt.ts` | 修复 2：合成用户消息扩展到所有模型 |

#### 验证步骤

1. 通过 Telegram 发送需要工具调用的请求（如搜索）
2. 验证 AI 在工具执行后继续生成分析内容
3. 检查日志无 JSON 解析错误
