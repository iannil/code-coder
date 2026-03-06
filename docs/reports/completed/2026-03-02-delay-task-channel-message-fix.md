# 延迟任务渠道消息修复

## 日期
2026-03-02

## 问题描述
用户通过 Telegram 发送 "1分钟后给我发消息" 时：
1. 系统成功创建了 delay task
2. 1分钟后任务执行成功（`lastStatus: "ok"`）
3. **但是消息没有发送到 Telegram**

## 根本原因分析

### 执行流程
```
Telegram 消息 → zero-channels → ccode Agent
                                    ↓
                            scheduler_delay_task
                                    ↓
                        创建 cron 任务 (type: agent)
                                    ↓
                              1分钟后
                                    ↓
                    zero-workflow 执行任务
                                    ↓
                    调用 /api/agent/invoke
                                    ↓
                    Agent 执行成功，结果在本地会话中
                                    ↓
                    ❌ 没有机制发送回 Telegram
```

### 问题点
`/api/agent/invoke` API 只是创建一个本地会话并运行 Agent，不会将结果发送回 IM 渠道。

## 解决方案

### 第一部分：添加 `channel_message` 命令类型

**TypeScript 端 (`packages/ccode/src/tool/scheduler.ts`)**:
- 在 `TaskCommandSchema` 中添加 `channel_message` 类型
- 在 `scheduler_delay_task` 中添加参数：
  - `channelType`: 渠道类型（telegram, feishu, wecom, dingtalk, discord, slack）
  - `channelId`: 渠道/聊天 ID
  - `channelMessage`: 要发送的消息文本

**Rust 端 (`services/zero-workflow/src/lib.rs`)**:
- 在 `CronCommand` 枚举中添加 `ChannelMessage` 变体
- 实现 `execute_channel_message_command` 函数
- 调用 `zero-channels` 的 `/api/v1/send` API 直接发送消息

### 第二部分：自动获取渠道上下文

为了让 Agent 能够自动获取当前的渠道信息（无需显式指定），实现了以下改进：

**1. 扩展 TaskContextRegistry (`packages/ccode/src/api/task/context.ts`)**:
- 新增 `contexts` Map 存储完整的 TaskContext
- 新增 `getContext(sessionID)` 方法获取上下文
- 新增 `getChannelInfo(sessionID)` 方法解析 `conversationId` 获取渠道类型和 ID

**2. 注册时传递上下文 (`packages/ccode/src/api/server/handlers/task.ts`)**:
- `TaskContextRegistry.register(sessionID, task.id, input.context)`

**3. 注入渠道信息到 Tool.Context (`packages/ccode/src/session/prompt.ts`)**:
- 在 `resolveTools` 中，从 TaskContextRegistry 获取渠道信息
- 将 `channelType` 和 `channelId` 注入到 `ctx.extra`

**4. 自动使用渠道信息 (`packages/ccode/src/tool/scheduler.ts`)**:
- 当用户使用 `message` 参数时，自动从 `ctx.extra` 获取渠道信息
- 如果存在渠道上下文，自动使用 `channel_message` 类型发送到 IM

### 修改的文件
1. `packages/ccode/src/tool/scheduler.ts` - 添加渠道参数和自动检测逻辑
2. `packages/ccode/src/api/server/handlers/scheduler.ts` - 更新序列化/反序列化逻辑
3. `packages/ccode/src/api/task/context.ts` - 扩展 TaskContextRegistry 存储上下文
4. `packages/ccode/src/api/server/handlers/task.ts` - 注册时传递上下文
5. `packages/ccode/src/session/prompt.ts` - 注入渠道信息到 Tool 上下文
6. `services/zero-workflow/src/lib.rs` - 添加 ChannelMessage 命令执行

## 使用方式

### 自动模式（推荐）
当用户通过 Telegram 发送 "1分钟后给我发消息" 时，Agent 只需调用：
```json
{
  "delayMinutes": 1,
  "message": "您好，这是一分钟后发送的消息！"
}
```
系统会自动从上下文检测 `channelType: "telegram"` 和 `channelId`，使用 `channel_message` 类型发送。

### 显式模式
如果需要发送到特定渠道：
```json
{
  "delayMinutes": 1,
  "channelType": "telegram",
  "channelId": "765318302",
  "channelMessage": "您好，这是一分钟后发送的消息！"
}
```

### 调用 Agent（仅本地会话）
```json
{
  "delayMinutes": 1,
  "agentName": "general",
  "prompt": "分析今天的市场数据"
}
```

## 验证步骤
1. 重新构建 Rust 服务: `cargo build --release -p zero-workflow`
2. 重启 zero-workflow 服务（杀掉进程，daemon 会自动重启）
3. 通过 Telegram 发送 "1分钟后给我发消息"
4. 验证消息在 1 分钟后发送回 Telegram

## 状态
- [x] TypeScript 代码修改
- [x] Rust 代码修改
- [x] TaskContextRegistry 扩展
- [x] Tool 上下文注入
- [x] 自动渠道检测
- [x] 编译验证通过
- [x] 服务重启
- [ ] 功能测试
