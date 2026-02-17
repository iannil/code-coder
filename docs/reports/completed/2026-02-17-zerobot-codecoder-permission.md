# ZeroBot 调用 CodeCoder 权限问题解决方案 - 完成报告

**完成日期**: 2026-02-17
**状态**: 已完成

## 概述

实现了 ZeroBot 通过 Telegram 调用 CodeCoder 的交互式权限确认系统，包括"始终批准"功能，允许用户将常用操作添加到持久化的允许列表中。

## 完成的修改

### 1. CodeCoder TypeScript 端

#### `packages/ccode/src/security/remote-policy.ts`
- 添加了用户允许列表的持久化功能
- 新增 `loadAllowlists()` - 从 `~/.codecoder/remote-allowlists.json` 加载
- 新增 `saveAllowlists()` - 保存到磁盘
- 修改 `allowForUser()`、`revokeForUser()`、`clearUserAllowlist()` 为 async 函数，自动保存

#### `packages/ccode/src/api/server/handlers/permission.ts`
- 新增 `addToAllowlist()` - POST /api/v1/permission/allowlist
- 新增 `getAllowlist()` - GET /api/v1/permission/allowlist/:userID
- 新增 `removeFromAllowlist()` - DELETE /api/v1/permission/allowlist/:userID/:tool
- 新增 `clearAllowlist()` - DELETE /api/v1/permission/allowlist/:userID

#### `packages/ccode/src/api/server/router.ts`
- 注册了新的允许列表管理路由

#### `packages/ccode/src/api/task/store.ts`
- 扩展 `PendingConfirmationInfo` 类型包含 permission 信息
- 更新 `setAwaitingApproval()` 接受 permission 参数
- 新增 `getPendingConfirmationInfo()` 返回完整确认信息

#### `packages/ccode/src/api/server/handlers/task.ts`
- 修改 `interactTask()` 支持 "always" 回复类型
- 当用户选择"始终批准"时，调用 `allowForUser()` 添加到允许列表
- 在 `createTask()` 中调用 `loadAllowlists()` 确保启动时加载

### 2. ZeroBot Rust 端

#### `services/zero-bot/src/agent/confirmation.rs`
- 新增 `ConfirmationResponse` 枚举：`Once`、`Always`、`Reject`
- 修改 `PendingConfirmation.responder` 使用 `ConfirmationResponse` 类型
- 修改 `request_confirmation_and_wait()` 返回 `ConfirmationResponse`
- 新增 `handle_confirmation_response_with_type()` 支持完整响应类型

#### `services/zero-bot/src/channels/mod.rs`
- 修改 Telegram 确认按钮：添加"✅ 始终批准"选项
- 修改 `handle_telegram_callbacks()` 处理 "always" 回调
- 使用 `handle_confirmation_response_with_type()` 传递完整响应

#### `services/zero-bot/src/tools/codecoder.rs`
- 修改确认处理逻辑支持 `ConfirmationResponse`
- 新增 `approve_task_with_reply()` 方法支持 "once"/"always" 回复
- 根据用户选择发送正确的 reply 类型到 CodeCoder API

## 数据流

```
用户点击 Telegram "始终批准" 按钮
    ↓
handle_telegram_callbacks() 解析 "always:request_id"
    ↓
confirmation::handle_confirmation_response_with_type(ConfirmationResponse::Always)
    ↓
codecoder.rs 收到 ConfirmationResponse::Always
    ↓
调用 approve_task_with_reply(task_id, request_id, "always")
    ↓
CodeCoder task.ts interactTask() 收到 reply="always"
    ↓
调用 allowForUser(userID, permission)
    ↓
保存到 ~/.codecoder/remote-allowlists.json
```

## 验证结果

- TypeScript 构建: ✅ 成功
- Rust 编译: ✅ 成功（无警告）

## 使用方法

1. 启动 CodeCoder API 服务器：
   ```bash
   cd ~/projects/agents-95e64f2e38 && bun dev serve
   ```

2. 启动 ZeroBot daemon：
   ```bash
   cd services/zero-bot && cargo run -- daemon
   ```

3. 通过 Telegram 发送消息，当遇到权限请求时：
   - 点击"✅ 批准"：仅批准本次操作
   - 点击"✅ 始终批准"：批准并记住偏好，后续同类操作自动批准
   - 点击"❌ 拒绝"：拒绝操作

## 持久化文件

用户允许列表存储在 `~/.codecoder/remote-allowlists.json`：
```json
{
  "telegram_123456": ["bash", "edit", "write"],
  "telegram_789012": ["bash"]
}
```

## 安全注意事项

- "始终批准"仅适用于受信任的用户
- 即使批准，操作仍受 `autonomy.workspace_only` 和 `forbidden_paths` 限制
- 建议定期审查用户允许列表
