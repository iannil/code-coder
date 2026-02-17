# Telegram Interactive Confirmation for CodeCoder

**完成时间**: 2026-02-17
**状态**: ✅ 已完成

## 背景

当 CodeCoder 执行需要用户授权的操作时（如打开浏览器、执行 shell 命令等），ZeroBot 之前直接返回错误而不是向用户询问授权。

### 原来的行为

```
Tool 'codecoder' succeeded (output len: 144)
🤖 Reply: 我无法直接控制您的浏览器来打开网站...
```

### 现在的行为

ZeroBot 向用户发送带有 inline keyboard 按钮的交互式消息，等待用户批准或拒绝后再继续执行。

## 实现内容

### 1. Telegram Channel 扩展 (`services/zero-bot/src/channels/telegram.rs`)

新增类型和方法：

```rust
// 新增类型
pub struct InlineButton { text, callback_data }
pub struct CallbackQuery { id, from_user_id, chat_id, message_id, data }

// 新增方法
async fn send_with_inline_keyboard(&self, chat_id, text, buttons) -> Result<i64>
async fn answer_callback_query(&self, id, text, show_alert) -> Result<()>
async fn edit_message_text(&self, chat_id, message_id, text) -> Result<()>
async fn listen_callback_queries(&self, tx) -> Result<()>
fn parse_callback_query(&self, json) -> Option<CallbackQuery>
```

### 2. Confirmation 模块扩展 (`services/zero-bot/src/agent/confirmation.rs`)

新增功能：

- 全局 `ConfirmationRegistry` 单例
- `NotificationSink` trait 扩展支持交互式确认
- `request_confirmation_and_wait()` 异步等待用户响应
- `handle_confirmation_response()` 处理用户回调

```rust
pub async fn request_confirmation_and_wait(
    channel: &str,
    user_id: &str,
    request_id: &str,
    permission: &str,
    message: &str,
    timeout_secs: Option<u64>,
) -> anyhow::Result<bool>
```

### 3. Channels 模块更新 (`services/zero-bot/src/channels/mod.rs`)

- `ChannelNotificationSink` 实现 `send_confirmation_request()` 方法
- 对 Telegram 使用 inline keyboard 按钮
- 对其他渠道回退到文本提示
- 启动时初始化 confirmation registry
- 为 Telegram 启动 callback query 监听器

### 4. CodeCoder Tool 更新 (`services/zero-bot/src/tools/codecoder.rs`)

修改 confirmation 处理逻辑：

```rust
// 如果不是 auto_approve，请求交互式确认
match confirmation::request_confirmation_and_wait(...).await {
    Ok(true) => {
        // 用户批准 -> 调用 approve API
        self.approve_task(task_id, &request_id).await?;
    }
    Ok(false) => {
        // 用户拒绝 -> 返回错误
        return Err(anyhow::anyhow!("用户拒绝了操作"));
    }
    Err(e) => {
        // 超时或系统未初始化 -> 回退到旧行为
        return Err(anyhow::anyhow!("需要授权..."));
    }
}
```

## 数据流

```
1. ZeroBot 调用 codecoder tool
2. CodeCoder 执行任务，需要权限
3. CodeCoder 发送 SSE confirmation 事件
4. ZeroBot codecoder tool 收到事件
5. ZeroBot 通过 ConfirmationRegistry 注册待处理请求
6. ZeroBot 向 Telegram 发送带按钮的消息
7. 用户点击 "✅ 批准" 或 "❌ 拒绝"
8. ZeroBot 收到 callback query
9. callback handler 通过 registry 通知等待的协程
10. codecoder tool 调用 CodeCoder API 批准/拒绝
11. 任务继续执行或中止
```

## 测试

### 单元测试 (969 tests passed)

- `inline_button_creation` - 按钮构建
- `telegram_send_with_inline_keyboard_fails_without_server` - 键盘消息发送
- `telegram_parse_callback_query_valid` - 回调解析
- `confirmation_registry_register_and_respond` - 注册表异步等待
- `confirmation_registry_multiple_concurrent` - 并发确认处理

### 集成测试步骤

```bash
# Terminal 1: 启动 CodeCoder API
cd packages/ccode && bun dev serve

# Terminal 2: 启动 ZeroBot
cd services/zero-bot && cargo run -- daemon

# Telegram: 发送需要授权的请求
"帮我打开携程网"

# 预期：
# 1. 收到带有 "✅ 批准" 和 "❌ 拒绝" 按钮的消息
# 2. 点击按钮后收到确认结果
# 3. 任务继续或终止
```

## 配置

无需额外配置。只要配置了 Telegram channel，交互式确认功能会自动启用。

确认超时时间默认为 120 秒（2 分钟）。

## 文件变更清单

| 文件 | 变更类型 | 描述 |
|------|----------|------|
| `src/channels/telegram.rs` | 修改 | 添加 inline keyboard 支持 |
| `src/agent/confirmation.rs` | 修改 | 添加全局注册表和异步等待 |
| `src/channels/mod.rs` | 修改 | 实现交互式 NotificationSink |
| `src/tools/codecoder.rs` | 修改 | 使用交互式确认流程 |

## 向后兼容性

- 如果 confirmation registry 或 notification sink 未初始化，回退到原来的错误返回行为
- `auto_approve=true` 仍然可用于自动批准所有请求
- 非 Telegram 渠道使用文本提示方式（需要用户回复 "approve {id}"）

## 修复记录

### 2026-02-17 - 修复消息接收问题

**问题**: Telegram 发送消息后 ZeroBot 收不到。

**原因**: 原实现使用两个独立的 `getUpdates` 轮询器（一个用于消息，一个用于回调），它们的 offset 会互相干扰。当一个轮询器获取更新并推进 offset 时，另一个轮询器会错过更新。

**解决方案**:
1. 改为单一轮询器，在 `listen()` 方法中同时监听 `message` 和 `callback_query`
2. 在 TelegramChannel 中添加 `callback_tx` 字段和 `set_callback_sender()` 方法
3. 在创建 TelegramChannel 时设置 callback sender（在 Arc 包装之前）
4. 主 listener 收到 callback 后发送到 callback handler 处理
5. 删除了不再需要的 `listen_callback_queries()` 方法和 `spawn_callback_listener()` 函数
