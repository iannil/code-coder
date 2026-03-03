# 消息模板配置指南

本文档说明如何配置 CodeCoder 的国际化消息模板。

## 概述

CodeCoder 支持自定义消息模板，允许您：

- 修改默认的中文消息
- 切换到其他语言（如英语）
- 自定义消息格式和表情符号

消息配置采用与关键词配置相同的模式：默认配置内置，用户配置可覆盖。

## 配置文件位置

```
~/.codecoder/messages.json
```

## 配置结构

```json
{
  "$schema": "https://code-coder.com/schemas/messages.json",
  "locale": "zh-CN",
  "version": "1.0.0",
  "messages": {
    "task": { ... },
    "approval": { ... },
    "status": { ... },
    "error": { ... },
    "auth": { ... },
    "search": { ... },
    "autonomous": { ... },
    "context": { ... }
  }
}
```

## 消息类别

### task - 任务生命周期消息

| 键名 | 默认值 | 说明 |
|------|--------|------|
| `acknowledged` | `🚀 收到，正在处理...` | 任务接收确认 |
| `start_processing` | `🚀 开始处理...\n📍 Trace: {trace_id}` | 任务开始处理 |
| `processing` | `🚀 处理中...` | 处理中状态 |
| `thinking` | `⏳ 思考中...` | AI 思考中 |
| `completed` | `✅ 处理完成` | 任务完成 |
| `failed` | `❌ 处理失败: {error}` | 任务失败 |

### approval - 审批消息

| 键名 | 默认值 | 说明 |
|------|--------|------|
| `title` | `🔐 CodeCoder 授权请求` | 授权请求标题 |
| `confirm_action` | `⚠️ **确认执行操作**` | 确认操作头部 |
| `approve` | `✅ 批准` | 批准按钮文本 |
| `approve_always` | `✅ 始终批准` | 始终批准按钮 |
| `reject` | `❌ 拒绝` | 拒绝按钮文本 |
| `pending` | `⏳ 待审批` | 待审批状态 |

### status - 状态指示器

| 键名 | 默认值 | 说明 |
|------|--------|------|
| `auto_approve` | `自动批准` | 自动批准决策 |
| `pending_approval` | `等待审批` | 等待审批决策 |
| `denied` | `拒绝` | 拒绝决策 |
| `answer_received` | `✅ 已收到回答` | 收到回答 |
| `answer_failed` | `❌ 回答失败` | 回答失败 |

### error - 错误消息

| 键名 | 默认值 | 说明 |
|------|--------|------|
| `load_failed` | `加载失败: {error}` | 加载失败 |
| `approve_failed` | `批准失败` | 批准操作失败 |
| `reject_failed` | `拒绝失败` | 拒绝操作失败 |
| `operation_failed` | `❌ 操作失败: {error}` | 通用操作失败 |
| `connection_lost` | `处理异常中断...请重试。` | 连接丢失 |

## 参数占位符

消息模板支持 `{key}` 格式的参数占位符：

```json
{
  "task": {
    "failed": "❌ 处理失败: {error}",
    "start_processing": "🚀 开始处理...\n📍 Trace: {trace_id}"
  }
}
```

可用占位符：
- `{error}` - 错误消息
- `{trace_id}` - 追踪 ID
- `{task_id}` - 任务 ID
- `{summary}` - 摘要内容
- `{approver}` - 审批人
- `{reason}` - 原因
- `{time}` - 时间
- `{count}` - 计数
- `{index}` - 索引

## 示例配置

### 切换到英文

```json
{
  "locale": "en-US",
  "messages": {
    "task": {
      "acknowledged": "🚀 Got it, processing...",
      "processing": "🚀 Processing...",
      "thinking": "⏳ Thinking...",
      "completed": "✅ Completed",
      "failed": "❌ Failed: {error}"
    },
    "approval": {
      "approve": "✅ Approve",
      "reject": "❌ Reject",
      "pending": "⏳ Pending"
    }
  }
}
```

### 自定义表情符号

```json
{
  "messages": {
    "task": {
      "acknowledged": "📨 收到啦，马上处理~",
      "completed": "🎉 搞定！",
      "failed": "💔 出错了: {error}"
    }
  }
}
```

### 极简模式

```json
{
  "messages": {
    "task": {
      "acknowledged": "OK, processing...",
      "completed": "Done.",
      "failed": "Error: {error}"
    },
    "approval": {
      "approve": "Yes",
      "reject": "No"
    }
  }
}
```

## 使用方式

### TypeScript

```typescript
import { t, taskT, approvalT } from "@/config/messages"

// 简单消息
const msg = t("task.acknowledged")
// => "🚀 收到，正在处理..."

// 带参数的消息
const error = t("task.failed", { error: "Network timeout" })
// => "❌ 处理失败: Network timeout"

// 使用作用域函数
const completed = taskT("completed")
const approve = approvalT("approve")
```

### Rust

```rust
use zero_common::messages::{messages, t};

// 简单消息
let msg = &messages().messages.task.acknowledged;

// 带参数的消息
let error = t("task.failed", &[("error", "Network timeout")]);
// => "❌ 处理失败: Network timeout"
```

## 验证配置

配置文件使用 JSON Schema 验证。在 VS Code 中，添加以下 `$schema` 字段以获得自动补全：

```json
{
  "$schema": "../../schemas/messages.schema.json",
  "locale": "zh-CN",
  ...
}
```

## 注意事项

1. **部分覆盖**: 只需配置您要修改的消息，未配置的将使用默认值
2. **热重载**: 修改配置后，需要重启服务才能生效
3. **编码**: 配置文件必须使用 UTF-8 编码
4. **表情符号**: 确保您的终端/IM 客户端支持所使用的表情符号

## 相关文档

- [关键词配置指南](./keywords-configuration.md) - Agent 触发词配置
- [配置系统概述](../architecture/CONFIG.md) - 统一配置系统
