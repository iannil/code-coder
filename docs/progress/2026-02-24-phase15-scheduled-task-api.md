# Phase 15: 定时任务 API 集成 (Scheduled Task API Integration)

**日期**: 2026-02-24
**状态**: ✅ 已完成

## 概述

实现 TypeScript API 来暴露 Rust `zero-workflow` 服务的定时任务管理功能，支持 Agent 任务调度和执行历史追踪。

## 核心设计

### 架构模式：代理到 Rust 服务

```
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│   TypeScript     │      │   zero-workflow  │      │     SQLite       │
│   API Handler    │─────▶│   Rust Service   │─────▶│   持久化存储      │
│   (Port 4400)    │      │   (Port 4412)    │      │                  │
└──────────────────┘      └──────────────────┘      └──────────────────┘
         │
         │ (fallback when Rust unavailable)
         ▼
┌──────────────────┐
│   In-Memory      │
│   Fallback       │
└──────────────────┘
```

**优点**:
- 单一数据源（Rust 服务的 SQLite）
- TypeScript 友好的 API 接口
- 优雅降级（Rust 服务不可用时使用内存模式）

### 任务命令类型

支持三种任务命令类型：

```typescript
type TaskCommand =
  | { type: "shell"; command: string }           // Shell 命令
  | { type: "agent"; agentName: string; prompt: string }  // Agent 调用
  | { type: "api"; endpoint: string; method: string; body?: object }  // API 调用
```

## 实现文件

| 文件 | 功能 |
|------|------|
| `src/api/server/handlers/scheduler.ts` | 定时任务管理 API (11 个端点) |
| `src/api/server/router.ts` | 注册 scheduler 路由 |
| `test/unit/api/scheduler.test.ts` | 57 个单元测试 |

## API 端点

### 任务管理

| 方法 | 端点 | 功能 |
|------|------|------|
| GET | `/api/v1/scheduler/tasks` | 列出所有任务 |
| POST | `/api/v1/scheduler/tasks` | 创建任务 |
| GET | `/api/v1/scheduler/tasks/:id` | 获取任务详情 |
| PUT | `/api/v1/scheduler/tasks/:id` | 更新任务 |
| DELETE | `/api/v1/scheduler/tasks/:id` | 删除任务 |
| POST | `/api/v1/scheduler/tasks/:id/run` | 手动触发执行 |

### 执行历史

| 方法 | 端点 | 功能 |
|------|------|------|
| GET | `/api/v1/scheduler/history` | 执行历史列表 |
| GET | `/api/v1/scheduler/history/:id` | 单次执行详情 |

### 配置与健康检查

| 方法 | 端点 | 功能 |
|------|------|------|
| GET | `/api/v1/scheduler/config` | 获取配置 |
| PUT | `/api/v1/scheduler/config` | 更新配置 |
| GET | `/api/v1/scheduler/health` | 健康检查 |

## Zod Schema 定义

```typescript
// 创建任务请求
const CreateTaskRequestSchema = z.object({
  id: z.string().min(1).max(64),
  name: z.string().min(1).max(128).optional(),
  description: z.string().max(512).optional(),
  expression: z.string().min(1),  // Cron 表达式
  command: TaskCommandSchema,
  enabled: z.boolean().default(true),
})

// 配置
const SchedulerConfigSchema = z.object({
  enabled: z.boolean(),
  defaultTimeZone: z.string().default("UTC"),
  maxConcurrentTasks: z.number().int().min(1).max(100).default(10),
  retryOnFailure: z.boolean().default(false),
  maxRetries: z.number().int().min(0).max(5).default(3),
  retryDelaySeconds: z.number().int().min(1).max(3600).default(60),
})
```

## 测试覆盖

| 测试类别 | 数量 | 内容 |
|---------|------|------|
| Schema 验证 | 34 | TaskCommand, ScheduledTask, CreateTaskRequest, ExecutionHistory, SchedulerConfig |
| 命令类型 | 9 | Shell, Agent, API 命令的各种场景 |
| Cron 表达式 | 2 | 标准和复杂 cron 表达式 |
| 边界情况 | 6 | Unicode, 特殊字符, 空输出, 长输出 |
| 业务逻辑 | 5 | 执行状态, 配置变体 |
| 回归测试 | 1 | null 字段处理 |

**总计**: 57 个测试全部通过

## 关键实现细节

### 1. 命令序列化

```typescript
function serializeCommand(cmd: TaskCommand): string {
  switch (cmd.type) {
    case "agent":
      return JSON.stringify({ type: "agent", agent: cmd.agentName, prompt: cmd.prompt })
    case "api":
      return JSON.stringify({ type: "api", endpoint: cmd.endpoint, method: cmd.method, body: cmd.body })
    case "shell":
      return cmd.command
  }
}
```

### 2. 任务执行

```typescript
// Agent 任务执行
case "agent": {
  const agentResponse = await fetch("http://127.0.0.1:4400/api/agent/invoke", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      agentId: command.agentName,
      prompt: command.prompt,
    }),
  })
  // ...
}
```

### 3. 优雅降级

当 Rust 服务不可用时，API 自动切换到内存模式：

```typescript
// Fallback to in-memory
if (result.status === 503) {
  const tasks = Array.from(inMemoryTasks.values()).map(/* ... */)
  return jsonResponse({
    success: true,
    data: tasks,
    meta: { source: "in-memory", warning: "Workflow service unavailable" },
  })
}
```

## 架构覆盖率更新

| 层级 | 之前 | 之后 |
|------|------|------|
| 中枢调度层 | 99% | 100% |

**完成**: 事件总线的定时唤醒能力 (ZB_Cron)

## 验证命令

```bash
# 类型检查
cd packages/ccode && bun run typecheck

# 单元测试
bun test test/unit/api/scheduler.test.ts

# 集成测试（需要启动 Rust 服务）
./ops.sh start zero-workflow
curl http://127.0.0.1:4400/api/v1/scheduler/health
```

## 使用示例

### 创建 Agent 定时任务

```bash
curl -X POST http://127.0.0.1:4400/api/v1/scheduler/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "id": "daily-review",
    "name": "每日代码审查",
    "description": "工作日上午 9 点进行代码审查",
    "expression": "0 9 * * 1-5",
    "command": {
      "type": "agent",
      "agentName": "@code-reviewer",
      "prompt": "审查过去 24 小时的代码变更"
    }
  }'
```

### 创建 Shell 定时任务

```bash
curl -X POST http://127.0.0.1:4400/api/v1/scheduler/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "id": "hourly-backup",
    "expression": "0 * * * *",
    "command": {
      "type": "shell",
      "command": "/opt/scripts/backup.sh"
    }
  }'
```

### 手动触发任务

```bash
curl -X POST http://127.0.0.1:4400/api/v1/scheduler/tasks/daily-review/run
```

## 后续优化方向

1. **持久化执行历史**: 将执行历史同步到 Rust 服务
2. **任务依赖**: 支持任务间的依赖关系
3. **失败通知**: 任务失败时发送通知
4. **Web UI**: 在管理台添加定时任务可视化界面

---

**完成时间**: 2026-02-24 (Phase 15)
**下一阶段**: 系统架构文档更新或新功能开发
