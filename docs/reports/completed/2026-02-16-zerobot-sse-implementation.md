# ZeroBot-CodeCoder SSE 通讯实现

**日期**: 2026-02-16
**状态**: 已完成

## 概述

将 ZeroBot 与 CodeCoder 的通讯方式从 HTTP 轮询升级为 Server-Sent Events (SSE)，实现实时流式响应。

## 变更前

```
ZeroBot                        CodeCoder
   |                              |
   |-- POST /api/sessions ------->|
   |<---- session_id -------------|
   |                              |
   |-- POST /messages ----------->|
   |<---- message_id -------------|
   |                              |
   |-- GET /messages (poll) ----->|  ← 每 2 秒轮询
   |<---- [] ---------------------|
   |-- GET /messages (poll) ----->|
   |<---- [] ---------------------|
   |-- GET /messages (poll) ----->|
   |<---- [response] -------------|
```

## 变更后

```
ZeroBot                        CodeCoder
   |                              |
   |-- POST /api/v1/tasks ------->|
   |<---- task_id ----------------|
   |                              |
   |-- GET /tasks/:id/events ---->|  ← SSE 连接
   |<==== progress ===============|  ← 实时推送
   |<==== progress ===============|
   |<==== confirmation ===========|  ← 权限请求
   |-- POST /tasks/:id/interact ->|  ← 自动/手动审批
   |<==== finish =================|  ← 完成
```

## 修改内容

**文件**: `services/zero-bot/src/tools/codecoder.rs`

### 1. 新增 Task API 类型

```rust
struct CreateTaskRequest {
    agent: String,
    prompt: String,
    model: Option<String>,
    context: Option<TaskContext>,
}

struct TaskContext {
    source: String,      // "remote"
    client_id: Option<String>, // "zerobot"
}
```

### 2. SSE 事件类型

```rust
enum TaskEvent {
    Progress(ProgressData),     // 进度更新
    Confirmation(ConfirmationData), // 权限请求
    Finish(FinishData),         // 完成/失败
}
```

### 3. SSE 解析器

```rust
fn parse_sse_event(text: &str) -> Option<TaskEvent>
```

支持两种格式：
- `event: message\ndata: {...}`
- `data: {...}`

### 4. 主要方法

| 方法 | 描述 |
|------|------|
| `create_task()` | 通过 Task API 创建任务 |
| `stream_task_events()` | 连接 SSE 流并处理事件 |
| `approve_task()` | 自动审批权限请求 |
| `poll_task_result()` | 回退: SSE 失败时轮询 |

### 5. 新增参数

| 参数 | 类型 | 描述 |
|------|------|------|
| `model` | `string?` | 指定模型 |
| `auto_approve` | `bool` | 自动审批权限请求 (默认 false) |

## 使用示例

```rust
// ZeroBot agent 调用 CodeCoder
let result = tool.execute(json!({
    "agent": "code-reviewer",
    "prompt": "Review the authentication module",
    "model": "anthropic/claude-sonnet-4",
    "auto_approve": true
})).await;
```

## 优势

1. **实时响应**: 无需 2 秒轮询延迟
2. **进度可见**: 实时显示任务进度
3. **权限控制**: 支持自动/手动审批
4. **回退机制**: SSE 失败时自动回退到轮询
5. **超时优化**:
   - HTTP: 5 分钟
   - SSE: 10 分钟

## 测试覆盖

新增 5 个 SSE 解析测试：
- `parse_sse_progress_event` - 进度事件解析
- `parse_sse_finish_event` - 完成事件解析
- `parse_sse_confirmation_event` - 确认事件解析
- `parse_sse_empty_data` - 空数据处理
- `parse_sse_invalid_json` - 无效 JSON 处理

## 架构对比

| 特性 | 轮询 (旧) | SSE (新) |
|------|----------|---------|
| 延迟 | 0-2 秒 | 实时 |
| 网络开销 | 高 (重复请求) | 低 (单连接) |
| 进度可见 | 否 | 是 |
| 权限处理 | 阻塞 | 交互式 |
| 复杂度 | 低 | 中 |
