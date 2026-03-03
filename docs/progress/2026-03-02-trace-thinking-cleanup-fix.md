# 修复：Trace完成后Thinking内容未删除

**日期**: 2026-03-03
**状态**: 已完成
**文件**:
- `services/zero-channels/src/progress.rs`
- `services/zero-channels/src/telegram/mod.rs`

## 问题描述

当用户通过IM（如Telegram）与agent交互时，trace完成后thinking（思考过程）内容没有被删除，导致最终的IM对话中包含了不应该保留的thinking内容。

### 问题根源分析

发现了三个层面的问题：

**问题1：Telegram Channel 不返回消息ID**
- `send_single_chunk` 方法返回 `Result<()>`，不返回消息ID
- `send` 方法返回 `uuid::Uuid::new_v4().to_string()` - 随机UUID而非实际Telegram消息ID
- 导致 `progress_message_id` 始终为 `None`

**问题2：Progress Message 未清理**
- `on_thought` 将thinking内容显示在progress message中
- 当task完成时，progress message未被清空

**问题3：独立思考消息未追踪**
- 当 `progress_message_id` 为 `None` 时，`on_thought` 将思考内容作为独立新消息发送
- 这些独立消息未被追踪，无法在完成时删除

## 解决方案

### 修改1：修复 Telegram Channel 返回真实消息ID

**文件**: `services/zero-channels/src/telegram/mod.rs`

1. 修改 `send_single_chunk` 返回 `anyhow::Result<i64>` - 实际Telegram消息ID
2. 从 Telegram API 响应中解析 `result.message_id`
3. 修改 `send` 方法返回第一个消息的ID

```rust
// send_single_chunk 现在返回消息ID
async fn send_single_chunk(&self, message: &str, chat_id: &str) -> anyhow::Result<i64> {
    // ... send message ...
    if resp.status().is_success() {
        let result: serde_json::Value = resp.json().await?;
        let message_id = result
            .get("result")
            .and_then(|r| r.get("message_id"))
            .and_then(|id| id.as_i64())
            .unwrap_or(0);
        return Ok(message_id);
    }
    // ...
}

// send 方法返回实际消息ID
async fn send(&self, message: OutgoingMessage) -> ChannelResult<String> {
    // ...
    let msg_id = self.send_single_chunk(chunk, &message.channel_id).await?;
    if i == 0 { captured_id = msg_id; }
    // ...
    Ok(first_message_id.to_string())
}
```

### 修改2：添加 `delete_message` 方法

**文件**: `services/zero-channels/src/telegram/mod.rs`

添加了 `delete_message` 方法，调用 Telegram API 的 `deleteMessage` 端点。

### 修改3：追踪思考消息ID

**文件**: `services/zero-channels/src/progress.rs`

1. 在 `MessageTracker` 结构体中添加 `thought_message_ids: Vec<i64>` 字段
2. 在 `on_thought` 的 fallback 路径中，捕获并存储消息ID

### 修改4：添加 `delete_telegram_message` 辅助方法

**文件**: `services/zero-channels/src/progress.rs`

在 `ImProgressHandler` 中添加了 `delete_telegram_message` 方法。

### 修改5：在 `on_finish` 中清理所有思考内容

**文件**: `services/zero-channels/src/progress.rs`

1. 编辑 progress message 显示 "✅ 处理完成"
2. 删除所有追踪到的独立思考消息

## 验证

1. **编译检查**: `cargo check` 通过
2. **测试**: 37个测试全部通过

## 验证方法（手动测试）

1. 启动服务：`./ops.sh start`
2. 通过Telegram发送一条消息给agent
3. 观察：
   - 在处理过程中，thinking内容可能显示
   - 当task完成后：
     - Progress message 被替换为 "✅ 处理完成"
     - 独立的思考消息被删除
   - Final output 作为新消息发送
4. 检查日志确认：
   - `progress_message_id` 应该有有效值（而非 `None`）
   - 看到 "cleaning up progress message" 日志
   - 看到 "deleting thought messages" 日志

## 风险评估

- **低风险**: 修改只影响IM消息的展示，不影响核心逻辑
- 所有清理操作都是 best-effort（使用 `let _ =` 忽略错误）
- Telegram API 可能因以下原因失败：
  - 消息已被用户删除
  - Rate limiting
  - 网络超时
- 失败不会影响主流程，final output 仍然会正常发送
