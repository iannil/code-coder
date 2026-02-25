# Telegram 会话控制命令实现

**日期**: 2026-02-24
**状态**: 已完成

## 概述

为 Telegram 渠道添加了 `/new` 和 `/compact` 命令支持，用于清空和压缩对话上下文。

## 实现内容

### 1. CodeCoder API 端点 (TypeScript)

文件: `packages/ccode/src/api/server/handlers/chat.ts`

添加了两个新端点:

- **POST /api/v1/chat/clear** - 清空会话上下文
  - 删除 `conversation_id` 到 `session_id` 的映射
  - 下次消息将创建新会话
  - 返回中英文确认消息

- **POST /api/v1/chat/compact** - 压缩会话上下文
  - 获取当前会话的所有消息
  - 使用 LLM 生成对话摘要
  - 创建新会话并以摘要作为初始上下文
  - 更新映射关系
  - 返回压缩结果

### 2. 路由注册 (TypeScript)

文件: `packages/ccode/src/api/server/router.ts`

```typescript
router.post("/api/v1/chat/clear", clearConversation)
router.post("/api/v1/chat/compact", compactConversation)
```

### 3. Bridge 命令解析 (Rust)

文件: `services/zero-channels/src/bridge.rs`

- 添加 `SessionCommand` 枚举（New, Compact）
- 添加 `parse_session_command()` 函数检测命令
- 添加 `handle_session_command()` 函数路由处理
- 添加 `call_clear_conversation()` 和 `call_compact_conversation()` API 调用

### 4. 命令格式

| 命令 | 别名 | 说明 |
|------|------|------|
| `/new` | `/clear` | 清空上下文，开始新对话 |
| `/compact` | `/summary` | 压缩上下文，保留摘要继续对话 |

### 5. 帮助信息更新

在 `@help` 帮助消息中添加了会话控制命令说明。

## 测试

添加了单元测试:
- `test_session_command_parsing` - 命令解析测试
- `test_session_command_with_whitespace` - 空白处理测试
- `test_agent_help_format` - 帮助消息测试（更新）

所有测试通过（46 个 bridge 相关测试）。

## 用户体验

1. **清空上下文 (`/new`)**:
   - 用户发送 `/new`
   - 系统返回: "✨ 上下文已清空，开始新对话！"
   - 下次消息开始全新对话

2. **压缩上下文 (`/compact`)**:
   - 用户发送 `/compact`
   - 系统返回: "🔄 正在压缩上下文..."
   - 完成后返回: "✅ 上下文已压缩，从 N 条消息精简为摘要。"
   - 对话继续，但上下文更精简

## 架构说明

```
Telegram → Bridge (Rust) → CodeCoder API (TypeScript)
    ↓           ↓                    ↓
  /new    parse_session_command   /api/v1/chat/clear
            ↓                          ↓
       handle_session_command    删除会话映射
            ↓                          ↓
       call_clear_conversation   返回确认消息
            ↓
       发送响应给用户
```

## 相关文件

- `packages/ccode/src/api/server/handlers/chat.ts` - API 处理器
- `packages/ccode/src/api/server/router.ts` - 路由配置
- `services/zero-channels/src/bridge.rs` - Bridge 实现
