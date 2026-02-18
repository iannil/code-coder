# Telegram 消息发送修复报告

**日期**：2026-02-19
**修复人**：Claude Code

## 问题描述

用户报告 ZeroBot 执行 CodeCoder 任务后，Telegram 没有收到返回的消息。日志显示任务成功完成，但消息发送部分静默失败。

## 根本原因分析

1. **`send` 方法未检查响应状态码**
   - 原代码只是发送请求，没有验证 Telegram API 的响应
   - 即使 API 返回错误（400, 413 等），代码也返回 `Ok(())`

2. **Markdown 解析错误**
   - 回复消息包含 `## `、`### ` 等 Markdown 语法
   - Telegram 的 Markdown 模式不支持这些语法
   - API 返回 400 错误："can't parse entities"

3. **消息长度超限**
   - Telegram 单条消息限制为 4096 字符
   - 个人资料汇总信息较长，可能超过限制

## 修复措施

### 1. 修改 `send` 方法（`telegram.rs:905-917`）

```rust
async fn send(&self, message: &str, chat_id: &str) -> anyhow::Result<()> {
    const MAX_MESSAGE_LEN: usize = 4096;
    let chunks = split_message(message, MAX_MESSAGE_LEN);
    for chunk in chunks {
        self.send_single_chunk(&chunk, chat_id).await?;
    }
    Ok(())
}
```

### 2. 新增 `send_single_chunk` 方法

- 检查响应状态码
- Markdown 解析失败时自动 fallback 到纯文本模式
- 添加详细日志便于诊断

### 3. 新增 `split_message` 辅助函数

- 超过 4096 字符的消息自动拆分
- 智能分割点（优先级：双换行 > 单换行 > 句号空格 > 空格）
- 后续块自动去除前导空白

## 测试

新增 7 个单元测试覆盖 `split_message` 函数：
- `split_message_short_message` - 短消息不分割
- `split_message_exact_limit` - 刚好达到限制
- `split_message_over_limit_at_newline` - 在换行处分割
- `split_message_over_limit_at_space` - 在空格处分割
- `split_message_no_natural_boundary` - 无自然分割点时强制分割
- `split_message_empty` - 空消息处理
- `split_message_trims_leading_whitespace_on_subsequent_chunks` - 后续块去除前导空白

## 附带修复

修复 `config/schema.rs` 中两个测试缺少 `tts` 字段的编译错误。

## 影响范围

- `services/zero-bot/src/channels/telegram.rs` - 主要修改
- `services/zero-bot/src/config/schema.rs` - 测试修复

## 验证

1. `cargo check` - 编译通过
2. `cargo test split_message` - 14/14 测试通过
3. clippy 无新增警告
