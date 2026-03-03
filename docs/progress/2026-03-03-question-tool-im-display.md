# 修复：Question工具在IM中不显示问题和选项

**日期**: 2026-03-03
**状态**: 已完成
**相关问题**: Trace完成后Thinking内容未删除 (同一修复)

## 问题描述

### 问题1: Question工具不显示
当agent调用`question`工具向用户提问时，IM（如Telegram）没有显示问题文本和选项，导致用户无法回答，最终task超时。

### 问题2: Thinking内容未清理
Trace完成后，thinking内容仍然显示在IM中。

## 根因分析

### Question工具问题
1. TypeScript端`Question.ask()`发布`question.asked`事件
2. TUI端有监听器来显示UI
3. **SSE/IM端没有任何监听器！** task handler只订阅了`PermissionNext.Event.Asked`，没有订阅`Question.Event.Asked`
4. Rust端没有Question事件类型和处理逻辑

### Thinking清理问题
1. Telegram Channel的`send`方法返回随机UUID而非实际消息ID
2. 导致`progress_message_id`始终为None
3. 清理逻辑无法执行

## 修复内容

### TypeScript端

**文件: `packages/ccode/src/api/task/types.ts`**
- 添加 `QuestionOption`、`QuestionInfo`、`QuestionEvent` 类型
- 将 `QuestionEvent` 加入 `TaskEvent` union

**文件: `packages/ccode/src/api/task/emitter.ts`**
- 添加 `question()` 方法用于发送问题事件

**文件: `packages/ccode/src/api/server/handlers/task.ts`**
- 导入 `Question` 模块
- 订阅 `Question.Event.Asked` 事件
- 转发问题到 SSE 流
- 在 finally 块中取消订阅

### Rust端

**文件: `services/zero-channels/src/sse.rs`**
- 添加 `QuestionOption`、`QuestionInfo`、`QuestionData` 结构体
- 在 `TaskEvent` enum 中添加 `Question` 变体

**文件: `services/zero-channels/src/telegram/mod.rs`**
- `send_single_chunk` 返回 `i64` (消息ID)
- `send` 方法返回实际Telegram消息ID
- 添加 `delete_message` 方法

**文件: `services/zero-channels/src/progress.rs`**
- 导入 `QuestionData` 和 `InlineButton`
- 在 `ProgressHandler` trait 添加 `on_question` 方法
- 实现 `on_question`: 发送带inline keyboard的问题消息
- 添加 `thought_message_ids` 追踪
- 添加 `delete_telegram_message` 方法
- 在 `on_finish` 清理progress和thought消息
- 在 `handle_event` 处理Question事件

**文件: `services/zero-channels/src/bridge.rs`**
- 在事件类型匹配中添加 `Question` 分支

## 问题展示流程

```
用户发送消息
    ↓
Agent调用question工具
    ↓
[TypeScript] Question.ask() → Bus.publish("question.asked")
    ↓
[TypeScript] task handler订阅 → TaskEmitter.question()
    ↓
[SSE Stream] question event
    ↓
[Rust] ImProgressHandler.on_question()
    ↓
[Telegram] 发送带inline keyboard的消息
    ↓
用户点击选项
    ↓
(需要后续实现callback query处理)
```

## 问题展示流程验证

1. **TypeScript编译**: `bun run typecheck` 通过
2. **Rust编译**: `cargo check` 通过
3. **Rust测试**: 364个测试全部通过

## Callback Query处理（已实现）

当用户点击Telegram inline button时的完整流程：

```
用户点击inline button
    ↓
[Telegram] POST callback_query to webhook
    ↓
[Rust routes.rs] handle_telegram_callback()
    ↓
解析callback data: q:{request_id}:{question_idx}:{option_idx}
    ↓
[Rust] reply_to_question() → POST /api/v1/questions/{requestId}/reply
    ↓
[TypeScript router.ts] 路由到 replyToQuestion handler
    ↓
[TypeScript task.ts] Question.reply() 恢复task执行
    ↓
[Rust] answer_callback_query() 确认按钮点击
    ↓
[Rust] edit_telegram_message() 更新消息显示用户选择
```

### Rust端实现（routes.rs）

**新增结构体:**
- `TelegramCallbackQuery` - 解析callback query
- `TelegramCallbackMessage` - 解析callback中的消息信息

**新增函数:**
- `handle_telegram_callback()` - 处理callback query入口
- `reply_to_question()` - 调用CodeCoder API回复问题
- `answer_callback_query()` - 向Telegram确认callback已处理
- `edit_telegram_message()` - 更新消息显示用户选择

### TypeScript端实现

**文件: `packages/ccode/src/api/server/router.ts`**
- 添加路由: `POST /api/v1/questions/:requestId/reply`

**文件: `packages/ccode/src/api/server/handlers/task.ts`**
- 添加 `replyToQuestion` handler
- 调用 `Question.reply()` 恢复task执行

## 验证

1. **TypeScript编译**: `bun turbo typecheck` 通过
2. **Rust编译**: `cargo check` 通过
3. **Rust测试**: 364个测试全部通过

## 风险评估

- **低风险**: 功能完整实现，包括问题展示和回复处理
- 用户可以在Telegram中看到问题和选项
- 用户点击选项后，task会恢复执行
