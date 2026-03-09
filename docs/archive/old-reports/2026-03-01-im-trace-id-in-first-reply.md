# IM 消息首条回复包含 TraceID

## 完成时间
2026-03-01

## 需求背景

用户反馈："回复im消息的第一条需要带上traceid，方便后续追踪"

当 IM 渠道（Telegram/Discord/Slack）收到用户消息并开始处理时，系统会发送一条开始消息 "🚀 开始处理..."。用户希望这条消息包含 TraceID，以便：

1. **问题追踪** - 用户报告问题时可以提供 TraceID
2. **日志关联** - 开发者可以快速定位对应的服务端日志
3. **全链路追踪** - 完成从用户侧到后端的完整追踪链路

## 实施方案

### 修改文件

`services/zero-channels/src/progress.rs`

### 修改内容

在 `ImProgressHandler::on_start()` 方法中，将开始消息从：

```rust
let text = "🚀 开始处理...";
```

修改为：

```rust
// Include trace_id in the first message for user tracking
let text = format!("🚀 开始处理...\n📍 Trace: {}", &msg.trace_id);
```

### 效果示例

用户在 Telegram 中发送消息后，会收到：

```
🚀 开始处理...
📍 Trace: abc123-def456-ghi789
```

## 技术说明

1. **TraceID 来源** - `msg.trace_id` 在 `ChannelMessage` 结构体中定义（`services/zero-channels/src/message.rs`）
2. **生成时机** - TraceID 在消息接收时由 `Bridge` 生成，使用 UUID v4 格式
3. **传播机制** - TraceID 通过 `MessageTracker` 在整个消息处理生命周期中传播
4. **日志关联** - 服务端日志已通过 `tracing::info!(trace_id = %msg.trace_id, ...)` 记录相同的 TraceID

## 验证方法

1. 启动服务：`./ops.sh start all`
2. 通过 Telegram 发送消息
3. 确认收到的第一条回复包含 "📍 Trace: xxx" 信息
4. 使用该 TraceID 在服务端日志中搜索验证关联

## 注意事项

- 当前 `zero-common/src/metrics.rs` 存在预存的编译错误（HashMap trait bounds 问题），与本次修改无关
- 建议后续修复 `zero-common` 的编译问题

## 相关文件

| 文件 | 作用 |
|------|------|
| `services/zero-channels/src/progress.rs` | 修改位置 - on_start 方法 |
| `services/zero-channels/src/message.rs` | TraceID 定义 - ChannelMessage.trace_id |
| `services/zero-channels/src/bridge.rs` | TraceID 生成 - UUID v4 |
