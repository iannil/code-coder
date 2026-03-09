# IM 任务执行系统重构：事件溯源 + 持久任务队列

## 实施进度

**开始时间**: 2026-03-01
**完成时间**: 2026-03-01
**当前状态**: ✅ 所有阶段完成 (Phase 1-4)

---

## 已完成工作

### Phase 1: 核心基础设施 ✅

#### Rust 侧

1. **Redis Streams 客户端** (`services/zero-common/src/redis.rs`)
   - 连接池管理 (ConnectionManager)
   - Stream 操作封装: XADD, XREAD, XREADGROUP, XACK
   - 消费者组管理: XGROUP CREATE, XCLAIM, XAUTOCLAIM
   - Hash 操作: HSET, HGETALL, HGET, HINCRBY
   - 健康检查和连接状态

2. **事件类型定义** (`services/zero-common/src/events.rs`)
   - TaskEvent 枚举 (task_created, task_started, thought, tool_use, progress, output, confirmation, agent_switch, heartbeat, debug_info, agent_info, skill_use, task_completed, task_failed)
   - StreamEvent 信封 (seq, timestamp, event, trace_id, span_id)
   - TaskState 投影 (状态机实现)
   - Hash 字段转换工具函数

3. **模块导出** (`services/zero-common/src/lib.rs`)
   - 新增 `pub mod events;` 和 `pub mod redis;`
   - 导出所有公共类型到 prelude

#### TypeScript 侧

1. **Redis Streams 客户端** (`packages/ccode/src/infrastructure/redis.ts`)
   - 使用 ioredis (与现有 ConversationStore 一致)
   - Stream 操作: xadd, xread, xreadgroup, xack, xpending, xclaim
   - Hash 操作: hset, hgetall, hget, hincrby
   - 单例模式和健康检查

2. **事件类型定义** (`packages/ccode/src/api/task/events.ts`)
   - StreamTaskEvent 联合类型 (扩展现有 TaskEvent)
   - StreamEventEnvelope (序列号 + 时间戳 + 追踪上下文)
   - TaskStateProjection (状态投影)
   - 类型转换: toStreamEvent(), fromStreamEvent()
   - 状态应用: applyEventToState(), createInitialState()

### Phase 2: 任务队列 ✅

#### Rust 侧

1. **任务调度器** (`services/zero-channels/src/task_dispatcher.rs`)
   - TaskRequest 数据结构 (从 ChannelMessage 转换)
   - 写入 tasks:pending 队列
   - 初始化任务状态 (tasks:state:{id})
   - 即时确认消息发送
   - Agent 检测逻辑 (detect_agent)

2. **依赖更新** (`services/zero-channels/Cargo.toml`)
   - 添加 `redis-backend` feature 到 zero-common

3. **Bridge 重构** (`services/zero-channels/src/bridge.rs`) ✅ (2026-03-01)
   - 添加 `use_redis_queue` 配置标志
   - 添加 `task_dispatcher` 和 `redis_client` 字段
   - 实现 `with_redis_queue()` 和 `init_redis_streams()` 方法
   - 新增 `process_streaming_redis()` 方法 (Redis Streams 模式)
   - 新增 `stream_event_to_sse()` 事件转换方法
   - 修改 `process_streaming_chat()` 为模式分发器:
     - 检查 `use_redis_queue` 配置
     - Redis 模式: 调用 `process_streaming_redis()`
     - HTTP/SSE 模式: 继续使用原有逻辑

#### TypeScript 侧

1. **Stream 消费者** (`packages/ccode/src/api/task/consumer.ts`)
   - XREADGROUP 从 pending 队列获取任务
   - 消费者组管理
   - 自动认领 (abandoned task claim)
   - 死信队列处理
   - 心跳机制

2. **TaskEmitter 重构** (`packages/ccode/src/api/task/emitter.ts`)
   - 双输出模式: SSE + Redis Streams
   - 事件序列号管理
   - 状态投影更新
   - 心跳事件 (仅 Redis)
   - 从 Stream 订阅 (subscribeFromStream)

### Phase 5: 配置 ✅

1. **配置定义** (`packages/ccode/src/config/config.ts`)
   - taskQueue 配置块:
     - backend: "redis" | "memory"
     - consumerGroup: 消费者组名
     - pendingTimeoutMs: 挂起超时
     - heartbeatIntervalMs: 心跳间隔
     - maxRetries: 最大重试次数
     - toolTimeoutMs: 单工具超时
     - globalTimeoutMs: 全局任务超时

### Phase 3: 可靠性增强 (部分完成)

#### Task #5: 心跳和超时策略 ✅ (2026-03-01)

1. **超时管理器** (`services/zero-channels/src/timeout.rs`)
   - TimeoutConfig: 可配置的超时参数
     - pending_timeout_ms: 挂起超时 (默认 5 分钟)
     - heartbeat_interval_ms: 心跳间隔 (默认 30 秒)
     - tool_timeout_ms: 单工具超时 (默认 1 分钟)
     - global_timeout_ms: 全局超时 (默认 30 分钟)
     - progress_warning_ms: 进度警告阈值 (默认 1 分钟)
   - TimeoutMonitor: 超时检查器
     - check_timeout(): 检查各类超时
     - should_warn_no_progress(): 检查是否需要警告
     - time_until_stale(): 计算剩余时间
   - TaskTimeoutState: 任务超时状态
     - 跟踪 started_at, last_heartbeat, last_progress
     - 跟踪当前工具执行 (current_tool)
   - TimeoutReason: 超时原因枚举
     - NoHeartbeat: 心跳超时
     - GlobalTimeout: 全局超时
     - ToolTimeout: 工具超时

2. **Bridge 集成** (`services/zero-channels/src/bridge.rs`)
   - 在 process_streaming_redis 中集成超时监控
   - 基于事件类型更新超时状态
   - Heartbeat 事件 → update_heartbeat()
   - Progress 事件 → update_progress()
   - ToolUse 事件 → start_tool() / end_tool()
   - 超时时发送友好的错误消息给用户

#### Task #9: 断点续传 ✅ (2026-03-01)

1. **检查点管理器** (`services/zero-channels/src/checkpoint.rs`)
   - Checkpoint 数据结构: last_id, task_id, updated_at, event_count
   - CheckpointManager:
     - load(): 加载断点位置
     - save(): 保存完整断点
     - update(): 更新断点位置
     - clear(): 清除断点 (任务完成时)
   - 检查点 TTL: 24 小时自动过期
   - Redis Hash 存储: `checkpoints:task:{task_id}`

2. **Bridge 集成** (`services/zero-channels/src/bridge.rs`)
   - 启动时加载断点: 从 checkpoint.last_id 继续
   - 周期性保存断点: 每 10 个事件保存一次
   - 任务完成时清除断点
   - 任务中断时保存断点用于后续恢复
   - 日志记录断点恢复状态

### Phase 4: IM 适配优化 ✅

#### Task #12: 重构 ImProgressHandler ✅ (2026-03-01)

1. **Redis Stream 事件消费** (`services/zero-channels/src/bridge.rs`)
   - `process_streaming_redis()` 方法从 Redis Stream 消费事件
   - 使用断点续传支持任务恢复
   - 超时监控和心跳检测

2. **统一事件转换** (`services/zero-channels/src/bridge.rs`)
   - `stream_event_to_sse()` 将 `StreamTaskEvent` 转换为 `crate::sse::TaskEvent`
   - 支持所有主要事件类型：
     - Thought → TaskEvent::Thought
     - ToolUse → TaskEvent::ToolUse
     - Progress → TaskEvent::Progress
     - Output → TaskEvent::Output
     - Confirmation → TaskEvent::Confirmation
     - DebugInfo → TaskEvent::DebugInfo
     - AgentInfo → TaskEvent::AgentInfo
     - SkillUse → TaskEvent::SkillUse
     - TaskCompleted/TaskFailed → TaskEvent::Finish

3. **平台特定格式化** (`services/zero-channels/src/progress.rs`)
   - `ImProgressHandler` 已支持多平台格式化
   - `format_debug_for_platform()` 根据渠道类型选择格式：
     - Telegram: HTML 格式
     - Slack: mrkdwn 格式
     - Discord: Markdown 格式
     - 其他: 纯文本格式

---

## 已完成工作汇总

所有 Phase 1-4 任务已完成 ✅

---

## 数据流架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           IM Channels                                   │
│  Telegram │ Discord │ Slack │ Feishu │ WeChat │ DingTalk │ WhatsApp     │
└─────────────────────────────┬───────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        zero-channels (Rust)                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │ Inbound      │───▶│ Task         │───▶│ Outbound     │              │
│  │ Normalizer   │    │ Dispatcher   │    │ Router       │              │
│  └──────────────┘    └──────┬───────┘    └──────▲───────┘              │
└─────────────────────────────┼───────────────────┼───────────────────────┘
                              │                   │
                              ▼                   │
┌─────────────────────────────────────────────────┼───────────────────────┐
│                     Redis Streams                │                      │
│  ┌────────────────────┐  ┌────────────────────┐ │ ┌──────────────────┐ │
│  │ tasks:pending      │  │ tasks:events:{id}  │─┘ │ tasks:state:{id} │ │
│  │ (任务入口队列)      │  │ (事件日志)          │   │ (状态投影)       │ │
│  └─────────┬──────────┘  └────────────────────┘   └──────────────────┘ │
└────────────┼────────────────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        ccode Task Worker (Bun)                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │ Stream       │───▶│ Agent        │───▶│ Event        │              │
│  │ Consumer     │    │ Executor     │    │ Publisher    │              │
│  └──────────────┘    └──────────────┘    └──────────────┘              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 关键文件清单

### 新增文件

| 文件路径 | 用途 |
|---------|------|
| `services/zero-common/src/redis.rs` | Rust Redis Streams 客户端 |
| `services/zero-common/src/events.rs` | Rust 事件类型定义 |
| `services/zero-channels/src/task_dispatcher.rs` | Rust 任务调度器 |
| `services/zero-channels/src/timeout.rs` | Rust 超时管理器 |
| `services/zero-channels/src/checkpoint.rs` | Rust 断点续传管理器 |
| `packages/ccode/src/infrastructure/redis.ts` | TS Redis Streams 客户端 |
| `packages/ccode/src/infrastructure/index.ts` | TS 基础设施模块导出 |
| `packages/ccode/src/api/task/events.ts` | TS 事件类型定义 |
| `packages/ccode/src/api/task/consumer.ts` | TS Stream 消费者 |

### 修改文件

| 文件路径 | 修改内容 |
|---------|---------|
| `services/zero-common/src/lib.rs` | 添加模块导出 |
| `services/zero-common/Cargo.toml` | 无修改 (已有 redis feature) |
| `services/zero-channels/src/lib.rs` | 添加 task_dispatcher 模块 |
| `services/zero-channels/Cargo.toml` | 添加 redis-backend feature |
| `services/zero-channels/src/bridge.rs` | 添加 Redis Streams 模式分发、process_streaming_redis 方法 |
| `packages/ccode/src/api/task/emitter.ts` | 添加 Redis Streams 输出 |
| `packages/ccode/src/config/config.ts` | 添加 taskQueue 配置 |

---

## 配置示例

```json
{
  "redis": {
    "url": "redis://localhost:4410",
    "keyPrefix": "codecoder:"
  },
  "taskQueue": {
    "backend": "redis",
    "consumerGroup": "ccode-workers",
    "pendingTimeoutMs": 300000,
    "heartbeatIntervalMs": 30000,
    "maxRetries": 3,
    "toolTimeoutMs": 60000,
    "globalTimeoutMs": 1800000
  }
}
```

---

## 下一步行动

1. ~~完成 bridge.rs 重构 (Task #10)~~ ✅
2. ~~实现心跳和超时策略 (Task #5)~~ ✅
3. 实现断点续传 (Task #9)
4. 重构 ImProgressHandler (Task #12)
5. 集成测试和渐进式迁移
