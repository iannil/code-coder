# CodeCoder & ZeroBot 深度融合实现报告

## 完成时间

2026-02-16

## 实施概述

实现了 CodeCoder 端的异步任务流模型（阶段一至三），为 ZeroBot 集成提供：

1. **Task API** - 完整的任务管理 REST API
2. **SSE 事件流** - 实时任务事件推送
3. **HITL 审批流程** - 远程调用的人工干预机制
4. **远程安全策略** - 危险操作识别与审批

## 新建文件

### Task 模块 (`packages/ccode/src/api/task/`)

| 文件 | 描述 |
|------|------|
| `types.ts` | Task、TaskEvent、TaskContext 等类型定义 |
| `store.ts` | 内存任务状态存储，支持 CRUD 和状态转换 |
| `emitter.ts` | Per-task SSE 事件发射器 |
| `index.ts` | 模块导出 |

### Security 模块 (`packages/ccode/src/security/`)

| 文件 | 描述 |
|------|------|
| `remote-policy.ts` | 远程安全策略，定义危险操作列表 |
| `index.ts` | 模块导出 |

### API Handler (`packages/ccode/src/api/server/handlers/`)

| 文件 | 描述 |
|------|------|
| `task.ts` | Task API 端点处理器 |

## 修改文件

| 文件 | 修改内容 |
|------|----------|
| `packages/ccode/src/id/id.ts` | 添加 `task` ID 前缀 |
| `packages/ccode/src/api/server/router.ts` | 注册 Task API 路由 |
| `packages/ccode/src/permission/next.ts` | 添加 `RemoteContext` 类型支持 |

## API 端点

| Method | Path | 用途 |
|--------|------|------|
| `POST` | `/api/v1/tasks` | 提交任务 |
| `GET` | `/api/v1/tasks` | 列出所有任务 |
| `GET` | `/api/v1/tasks/:id` | 获取任务状态 |
| `GET` | `/api/v1/tasks/:id/events` | SSE 事件流 |
| `POST` | `/api/v1/tasks/:id/interact` | 人工干预 |
| `DELETE` | `/api/v1/tasks/:id` | 删除任务 |

## 核心类型

### TaskEvent

```typescript
type TaskEvent =
  | { type: "thought"; data: string }
  | { type: "tool_use"; data: { tool: string; args: any; result?: any } }
  | { type: "output"; data: string }
  | { type: "confirmation"; data: ConfirmationRequest }
  | { type: "progress"; data: { stage: string; message: string; percentage?: number } }
  | { type: "finish"; data: { success: boolean; output?: string; error?: string } }
```

### TaskContext

```typescript
interface TaskContext {
  userID: string        // ZeroBot 用户标识
  platform: string      // telegram/discord/slack
  chatHistory?: any[]   // 最近对话
  source: "remote"      // 标记为远程调用
}
```

## 远程安全策略

### 危险操作（需要审批）

- **文件修改**: write, edit, patch, multiedit, delete, move, rename
- **Shell 执行**: bash, shell, exec, run
- **Git 修改**: git_push, git_commit, git_reset, git_checkout
- **网络操作**: fetch, curl, http

### 安全操作（无需审批）

- read, view, search, grep, find, list
- git_status, git_log, git_diff

## 工作流程

```
1. ZeroBot 调用 POST /api/v1/tasks
   ↓
2. CodeCoder 创建 Task，返回 taskID
   ↓
3. ZeroBot 订阅 GET /api/v1/tasks/:id/events (SSE)
   ↓
4. CodeCoder 执行 Agent
   ├── 发射 progress 事件
   ├── 发射 tool_use 事件
   ├── 遇到危险操作 → 发射 confirmation 事件
   │   ↓
   │   ZeroBot 显示审批按钮
   │   ↓
   │   用户点击 → POST /api/v1/tasks/:id/interact
   │   ↓
   │   继续执行
   ↓
5. 完成 → 发射 finish 事件
```

## 类型检查

所有新代码通过 TypeScript 类型检查。

存在的类型错误（`src/provider/provider.ts`）是预先存在的问题，与本次实现无关。

## 下一步（ZeroBot 端）

1. **阶段四**: 实现 SSE 客户端（Rust）
   - `services/zero-bot/src/sse/` 模块
   - 升级 `CodeCoderTool` 为 SSE 模式
   - 保留轮询作为 fallback

2. **阶段五**: 记忆集成
   - 用户映射配置
   - 记忆元数据注入

## 测试验证

可通过以下方式验证实现：

```bash
# 启动 API 服务器
bun dev serve

# 提交任务
curl -X POST http://localhost:4096/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "agent": "build",
    "prompt": "分析当前目录结构",
    "context": {
      "userID": "test_user",
      "platform": "test",
      "source": "remote"
    }
  }'

# 订阅事件流
curl http://localhost:4096/api/v1/tasks/{taskID}/events

# 获取任务状态
curl http://localhost:4096/api/v1/tasks/{taskID}
```

## 总结

CodeCoder 端的异步任务流模型已完成实现：

- ✅ Task API 基础设施
- ✅ SSE 事件流
- ✅ HITL 审批流程
- ✅ 远程安全策略
- ✅ Permission 系统集成

ZeroBot 端（阶段四、五）将在后续实现。
