# IM 消息处理过程信息增强

**完成时间**: 2026-02-26

## 概述

实现了 IM（Telegram 等）消息回复中的详细处理过程信息显示，包括思考过程、工具调用详情、实时输出和执行摘要。

## 实现内容

### ccode 侧修改

#### 1. 新建 TaskContextRegistry
**文件**: `packages/ccode/src/api/task/context.ts`

- 使用 `Instance.state` 实现 session → taskID 映射
- 提供 `register()`, `getTaskID()`, `unregister()` 方法
- 生命周期与项目实例绑定

#### 2. 修改 TaskHandler
**文件**: `packages/ccode/src/api/server/handlers/task.ts`

- 在任务执行前注册 session → taskID 映射
- 在任务完成后（finally 块）清理映射

#### 3. 修改 SessionProcessor
**文件**: `packages/ccode/src/session/processor.ts`

- 添加可选的 `taskID` 参数
- 在流处理中发射 SSE 事件：
  - `reasoning-start` → `TaskEmitter.thought()`
  - `reasoning-delta` → 节流发送思考内容
  - `tool-call` → `TaskEmitter.toolUse()`
  - `tool-result` → `TaskEmitter.toolUse()` with result
  - `text-delta` → 节流发送输出内容

#### 4. 修改 SessionPrompt
**文件**: `packages/ccode/src/session/prompt.ts`

- 在 `loop()` 函数中获取 taskID
- 传递给 `SessionProcessor.create()`

### zero-channels 侧修改

#### 5. 扩展 ProgressHandler trait
**文件**: `services/zero-channels/src/progress.rs`

- 添加 `on_thought()` 方法
- 添加 `on_output()` 方法

#### 6. 增强 ImProgressHandler
**文件**: `services/zero-channels/src/progress.rs`

- 添加工具使用统计 (`tools_used: HashMap`)
- 添加任务开始时间记录 (`task_start: Instant`)
- 添加思考节流 (`thought_throttle_interval`)
- 实现思考内容格式化（💭 前缀，200 字符截断）
- 实现输出内容格式化（📝 前缀，300 字符截断）
- 实现执行摘要生成（耗时、工具调用统计）
- 在 `on_finish()` 中附加执行摘要

#### 7. 更新事件分发
**文件**: `services/zero-channels/src/progress.rs`

- 修改 `handle_event()` 处理 `Thought` 和 `Output` 事件
- 之前这些事件被忽略，现在会显示给用户

## 验证方式

通过 Telegram 发送 `@macro 解读PMI数据`，观察显示：

```
🚀 开始处理...
💭 开始思考...
⚡ web_search query: "PMI data 2026"
📄 正在读取文件...
✅ 处理完成

📊 执行摘要
⏱ 耗时: 12.3s
🔧 工具调用: 5 次
   • web_search: 2
   • read: 2
   • grep: 1
```

## 性能考虑

- **Thought 事件**: 每 200 字符发送一次，且节流 500ms
- **Output 事件**: 每 100 字符发送一次，且节流 1s
- **工具参数**: 截断至 200 字符显示，避免发送大量数据

## 相关文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `packages/ccode/src/api/task/context.ts` | 新建 | 任务上下文注册表 |
| `packages/ccode/src/api/server/handlers/task.ts` | 修改 | 注册/清理 task 上下文 |
| `packages/ccode/src/session/processor.ts` | 修改 | 添加 taskID 参数和 SSE 事件发射 |
| `packages/ccode/src/session/prompt.ts` | 修改 | 传递 taskID 到 SessionProcessor |
| `services/zero-channels/src/progress.rs` | 修改 | 处理 Thought/Output 事件，生成摘要 |
