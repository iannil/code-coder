# 修复：Agent 定时任务执行后推送回 IM 渠道

**日期**: 2026-03-03
**状态**: 待验证
**优先级**: 高

## 问题描述

用户通过 Telegram 创建延迟任务（如 "3分钟后让 @macro 整理早间新闻"），任务成功执行但结果没有推送回 Telegram。

**根本原因**: 当创建 `agent` 类型的定时任务时，任务命令中没有保存回调渠道信息。Agent 执行后的输出只保存在本地 session，不会发送到原始 IM 渠道。

## 修改内容

### 1. Rust: CronCommand::Agent 新增回调字段

**文件**: `services/zero-workflow/src/lib.rs:492-503`

```rust
CronCommand::Agent {
    agent: String,
    prompt: String,
    #[serde(default)]
    callback_channel_type: Option<String>,
    #[serde(default)]
    callback_channel_id: Option<String>,
}
```

### 2. Rust: execute_agent_command 支持回调

**文件**: `services/zero-workflow/src/lib.rs:554-650`

- 添加 `callback_channel_type` 和 `callback_channel_id` 参数
- Agent 执行完成后，如果有回调渠道配置，调用 `execute_channel_message_command` 发送结果

### 3. TypeScript API Handler: TaskCommandSchema 更新

**文件**: `packages/ccode/src/api/server/handlers/scheduler.ts`

- `TaskCommandSchema` agent 类型添加 `callbackChannelType` 和 `callbackChannelId` 可选字段
- `serializeCommand` 序列化时包含回调字段
- `parseCommand` 解析时提取回调字段

### 4. TypeScript Tool: 自动注入回调渠道

**文件**: `packages/ccode/src/tool/scheduler.ts`

- `TaskCommandSchema` 添加回调字段描述
- `scheduler_create_task` 和 `scheduler_delay_task` 创建 agent 任务时自动从上下文注入回调渠道信息

## 数据流

```
用户 (Telegram)
    ↓ "3分钟后让 @macro 整理新闻"
scheduler_delay_task (ctx.extra 包含 channelType/channelId)
    ↓ 创建 command: { type: "agent", ..., callbackChannelType: "telegram", callbackChannelId: "xxx" }
zero-workflow (Rust)
    ↓ 3分钟后执行
execute_agent_command
    ↓ 调用 CodeCoder API 获取 Agent 响应
execute_channel_message_command
    ↓ 发送响应到 Telegram
用户收到 @macro 的分析结果
```

## 验证步骤

1. 重启服务：
   ```bash
   ./ops.sh stop && ./ops.sh start all
   ```

2. 通过 Telegram 测试：
   - 发送 "1分钟后让 @macro 分析今天的市场"
   - 等待任务执行
   - 确认收到 Agent 响应消息

3. 检查日志确认回调发送：
   ```bash
   tail -f ~/.codecoder/logs/zero-workflow.log | grep -E "callback|channel"
   ```

## 构建状态

- [x] Rust 编译通过 (`cargo check` / `./ops.sh build rust`)
- [x] TypeScript 类型检查通过 (`bun tsc --noEmit`)
- [ ] 集成测试
- [ ] 用户验收测试
