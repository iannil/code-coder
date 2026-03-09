# Phase 4.4 完成报告：TUI 后端抽象层

**日期**: 2026-03-05
**状态**: ✅ 完成

## 概述

本阶段完成了 Phase 4 的最后一步 - 重构 ccode 启动流程，使 TUI 可以通过 IPC 与 zero-cli 通信。采用适配器模式创建了统一的后端接口，支持两种模式切换。

## 实现内容

### 新增文件

#### 1. `packages/ccode/src/cli/cmd/tui/backend/index.ts`

**后端接口定义**

```typescript
export interface TuiBackend {
  readonly mode: BackendMode  // "worker" | "ipc"
  readonly events: EventSource
  readonly rpc: RpcClient
  reload(): Promise<void>
  shutdown(): Promise<void>
  isConnected(): boolean
}
```

核心设计：
- `events`: 订阅后端事件（Bus events）
- `rpc`: SDK 风格的 API 调用
- 生命周期方法：`reload()`, `shutdown()`

#### 2. `packages/ccode/src/cli/cmd/tui/backend/worker.ts`

**Worker 后端适配器**

- 封装现有 `worker.ts` 和 RPC 逻辑
- 在独立 Web Worker 线程中运行 LocalAPI
- 实现 `TuiBackend` 接口

```typescript
export async function createWorkerBackend(options?: WorkerBackendOptions): Promise<WorkerBackend>
```

#### 3. `packages/ccode/src/cli/cmd/tui/backend/ipc.ts`

**IPC 后端适配器**

- 使用 `IpcClient` 连接 zero-cli
- 支持自动启动 zero-cli
- 实现方法路由：将 SDK 调用映射到 IPC 协议
- 提供未实现方法的 stub 响应

```typescript
export async function createIpcBackend(options?: IpcBackendOptions): Promise<IpcBackend>
```

### 修改文件

#### 4. `packages/ccode/src/cli/cmd/tui/thread.ts`

**TUI 启动命令**

新增命令行选项：
```bash
ccode --backend worker  # 默认：使用 Web Worker
ccode --backend ipc     # IPC 模式：连接 zero-cli
ccode --backend ipc --socket /path/to/socket  # 自定义 socket 路径
```

重构要点：
- 移除硬编码的 Worker 创建逻辑
- 使用 `createWorkerBackend()` 或 `createIpcBackend()` 工厂函数
- 统一通过 `backend.events` 和 `backend.rpc` 访问后端

## 架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         TUI Thread (thread.ts)                              │
│                              │                                              │
│                    ┌─────────┴─────────┐                                    │
│                    │    TuiBackend     │  (Unified Interface)               │
│                    └─────────┬─────────┘                                    │
│               ┌──────────────┴──────────────┐                               │
│               ▼                             ▼                               │
│    ┌─────────────────────┐       ┌─────────────────────┐                   │
│    │   WorkerBackend     │       │    IpcBackend       │                   │
│    │  (Web Worker + RPC) │       │  (Unix Socket IPC)  │                   │
│    └─────────────────────┘       └─────────────────────┘                   │
│               │                             │                               │
│               ▼                             ▼                               │
│    ┌─────────────────────┐       ┌─────────────────────┐                   │
│    │     worker.ts       │       │     zero-cli        │                   │
│    │     (LocalAPI)      │       │   (serve-ipc)       │                   │
│    └─────────────────────┘       └─────────────────────┘                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 技术要点

### 1. 适配器模式

两种后端实现相同的 `TuiBackend` 接口，允许在运行时切换：
- Worker 后端：零配置，本地执行
- IPC 后端：需要 zero-cli，支持分布式架构

### 2. 方法路由

IPC 后端将 SDK 风格的调用路由到 IPC 协议：
```typescript
"session.list" → client.listSessions()
"session.get"  → client.getSession(id)
"config.get"   → stubConfigGet()  // 未实现的方法返回 stub
```

### 3. 事件映射

IPC 通知转换为 TUI 事件格式：
```typescript
"session_update" → { type: "session.update", properties: {...} }
"stream_token"   → { type: "session.stream", properties: {...} }
```

## 验证

```bash
# TypeScript 类型检查通过
bun turbo typecheck --filter=ccode
# Tasks: 1 successful, 1 total

# Worker 模式测试
bun dev  # 应正常启动 TUI

# IPC 模式测试 (需要 zero-cli)
bun dev -- --backend ipc
```

## Phase 4 总览

| 子任务 | 状态 | 描述 |
|--------|------|------|
| 4.1 IPC Server (Rust) | ✅ | `services/zero-cli/src/ipc/` |
| 4.2 serve-ipc 命令 | ✅ | main.rs, lib.rs |
| 4.3 TypeScript IPC Client | ✅ | `packages/ccode/src/ipc/` |
| 4.4 TUI 后端抽象层 | ✅ | `packages/ccode/src/cli/cmd/tui/backend/` |

## 后续工作

1. **IPC 方法完善**: 随着 zero-cli 实现更多功能，替换 stub 响应
2. **测试覆盖**: 添加后端适配器的单元测试
3. **性能优化**: 评估 IPC 模式的延迟和吞吐量
4. **错误处理**: 增强连接断开重连的用户体验

## 文件清单

```
packages/ccode/src/cli/cmd/tui/backend/
├── index.ts    # 接口定义和类型导出
├── worker.ts   # Worker 后端适配器
└── ipc.ts      # IPC 后端适配器

packages/ccode/src/cli/cmd/tui/
└── thread.ts   # 修改：添加 --backend 选项
```
