# packages/ccode Rust 迁移实施计划

**创建时间**: 2026-03-04
**状态**: 进行中
**负责**: AI Agent

---

## 目录

1. [概述](#概述)
2. [架构设计](#架构设计)
3. [P0: 消除重复](#p0-消除重复)
4. [P1: 性能优化](#p1-性能优化)
5. [P2: 安全与计算](#p2-安全与计算)
6. [API 契约](#api-契约)
7. [测试与验证](#测试与验证)
8. [回滚策略](#回滚策略)

---

## 概述

### 迁移原则

基于项目核心哲学：**高确定性任务用 Rust 保证效率；高不确定性任务用 TypeScript/LLM 保证正确反应**。

### 目标

- **消除代码重复**: MCP 系统在 TS 和 Rust 中都有实现
- **提升性能**: I/O 密集型任务迁移到 Rust
- **增强可靠性**: SQLite 持久化替代内存状态
- **统一架构**: 所有服务通过 HTTP/IPC 通信

### 不迁移的模块

以下模块**保留在 TypeScript**，原因标注：

| 模块 | 原因 |
|------|------|
| Agent 核心 | 需要 LLM 推理，与 AI 紧密耦合 |
| TUI/CLI | Solid.js + OpenTUI，UI 密集 |
| 配置管理 | 复杂 JSONC 解析，TS 生态更成熟 |
| API Server | Bun.serve 性能已足够，与 Agent 耦合 |

---

## 架构设计

### 最终目标架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        packages/ccode (TypeScript)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Agent 核心    │  │   TUI/CLI    │  │ 配置管理      │  │ API Gateway  │   │
│  │ (LLM 推理)   │  │  (Solid.js)  │  │ (JSONC)      │  │  (Bun.serve) │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                 │                 │             │
│         ▼                 ▼                 ▼                 ▼             │
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │                    HTTP/IPC 通信层 (统一接口)                           ││
│  └────────────────────────────┬───────────────────────────────────────────┘│
└───────────────────────────────┼──────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        services/zero-* (Rust)                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────┐│
│  │   MCP    │ │Scheduler │ │ Storage  │ │ Tracing  │ │  RBAC    │ │Vector││
│  │ Manager  │ │(SQLite)  │ │ Service  │ │ Service  │ │ Engine   │ │Search││
│  │  :4420   │ │  :4432   │ │  :4440   │ │  :4441   │ │  :4442   │ │:4443 ││
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

### 端口分配

**核心服务 (4400-4409)** - 已分配
**基础设施服务 (4410-4419)** - 已分配
**协议服务 (4420-4429)** - MCP
**Rust 微服务 (4430-4439)** - 已分配
**新增迁移服务 (4440-4449)**:
- 4440: Storage Service (新增)
- 4441: Tracing Service (新增)
- 4442: RBAC Service (新增)
- 4443: Vector Search Service (可使用 zero-memory 端口)

### 通信协议

```typescript
// 统一的 HTTP 客户端封装
interface RustServiceClient {
  // 基础通信
  request<T>(path: string, data?: unknown): Promise<T>
  stream(path: string): ReadableStream

  // 服务特定接口
  mcp: McpServiceClient
  scheduler: SchedulerServiceClient
  storage: StorageServiceClient
  tracing: TracingServiceClient
  rbac: RbacServiceClient
  vector: VectorServiceClient
}
```

---

## P0: 消除重复

### P0.1: MCP 系统迁移

**优先级**: 极高
**复杂度**: 中
**预期收益**: 消除 ~2000 行重复代码

#### 现状分析

| 项目 | TypeScript | Rust |
|------|-----------|------|
| 文件数 | 5 个文件 | 已完整实现 |
| 代码量 | ~2,090 行 | 完整 MCP 协议 |
| 功能 | Client + Server + OAuth | Client + Server + 统一工具注册表 |
| 传输 | HTTP/SSE/Stdio | HTTP/SSE/Stdio |

#### 迁移方案

**步骤 1**: 暴露 Rust MCP 服务为 HTTP API

```rust
// services/zero-cli/src/mcp/http.rs (新建)

use axum::{Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct McpStatusResponse {
    pub servers: Vec<ServerStatus>,
}

#[derive(Serialize)]
pub struct ServerStatus {
    pub name: String,
    pub status: String,  // "connected", "disabled", "needs_auth"
    pub tools_count: usize,
}

pub fn mcp_http_routes() -> Router {
    Router::new()
        .route("/api/mcp/status", axum::routing::get(mcp_status))
        .route("/api/mcp/servers/:name/tools", axum::routing::get(list_tools))
        .route("/api/mcp/servers/:name/connect", axum::routing::post(connect_server))
        .route("/api/mcp/servers/:name/disconnect", axum::routing::post(disconnect_server))
        .route("/api/mcp/servers/:name/auth", axum::routing::post(start_auth))
}

async fn mcp_status() -> Json<McpStatusResponse> {
    // 查询所有 MCP 服务器状态
}
```

**步骤 2**: TypeScript 端适配层

```typescript
// packages/ccode/src/mcp/rust-client.ts (新建)

import type { Config } from "../config/config"

export namespace MCP.Rust {
  interface ServerStatus {
    name: string
    status: "connected" | "disabled" | "needs_auth"
    tools_count: number
  }

  /**
   * 调用 Rust MCP 服务获取状态
   */
  export async function status(): Promise<Record<string, Status>> {
    const response = await fetch("http://localhost:4420/api/mcp/status")
    const data = await response.json()

    // 转换为兼容格式
    return Object.fromEntries(
      data.servers.map((s: ServerStatus) => [s.name, { status: s.status }])
    )
  }

  /**
   * 获取工具列表（从 Rust 服务）
   */
  export async function tools(): Promise<Record<string, Tool>> {
    const response = await fetch("http://localhost:4420/api/mcp/tools")
    return response.json()
  }
}
```

**步骤 3**: 渐进式切换

```typescript
// packages/ccode/src/mcp/index.ts (修改)

import { Config } from "../config/config"
import * as RustMCP from "./rust-client"

// 配置开关
const USE_RUST_MCP = true

export async function status() {
  if (USE_RUST_MCP) {
    return RustMCP.status()
  }
  // 保留原有实现作为后备
  return originalStatus()
}
```

#### 验收标准

- [ ] Rust MCP 服务 HTTP API 完整实现
- [ ] TypeScript 适配层通过所有测试
- [ ] 功能对等：所有现有 MCP 功能可用
- [ ] 性能基准：工具调用延迟 < 100ms
- [ ] 错误处理：Rust 服务不可用时降级到 TS 实现

---

### P0.2: 任务调度迁移

**优先级**: 极高
**复杂度**: 低
**预期收益**: 持久化，稳定性提升

#### 现状对比

| 特性 | TypeScript | Rust |
|------|-----------|------|
| 持久化 | ❌ 内存 | ✅ SQLite |
| Cron 支持 | 基础 interval | 完整 (5/6/7 字段) |
| 执行历史 | ❌ | ✅ |
| 错误处理 | 基础 | 完善 |

#### 迁移方案

**步骤 1**: 暴露 Rust Scheduler HTTP API

```rust
// services/zero-workflow/src/http.rs (新建)

use axum::{Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct ScheduleTaskResponse {
    pub id: String,
    pub next_run: String,
}

pub fn scheduler_http_routes() -> Router {
    Router::new()
        .route("/api/scheduler/tasks", axum::routing::get(list_tasks))
        .route("/api/scheduler/tasks", axum::routing::post(add_task))
        .route("/api/scheduler/tasks/:id", axum::routing::delete(remove_task))
        .route("/api/scheduler/tasks/:id/history", axum::routing::get(task_history))
}

async fn add_task(
    Json(task): Json<CronTask>,
) -> Json<ScheduleTaskResponse> {
    // 添加任务到调度器
}
```

**步骤 2**: TypeScript 适配层

```typescript
// packages/ccode/src/scheduler/rust-client.ts (新建)

export namespace Scheduler.Rust {
  export interface Task {
    id: string
    expression: string  // cron 表达式
    command: string
    description?: string
    next_run: string
    last_run?: string
  }

  export async function register(task: Task): Promise<void> {
    await fetch("http://localhost:4432/api/scheduler/tasks", {
      method: "POST",
      body: JSON.stringify(task),
    })
  }

  export async function list(): Promise<Task[]> {
    const response = await fetch("http://localhost:4432/api/scheduler/tasks")
    return response.json()
  }
}
```

**步骤 3**: 兼容性包装

```typescript
// packages/ccode/src/scheduler/index.ts (修改)

import * as RustScheduler from "./rust-client"

// 保留原有接口，内部调用 Rust
export function register(task: Task) {
  if (task.interval) {
    // 转换 interval 为 cron 表达式
    const expression = intervalToCron(task.interval)
    return RustScheduler.register({
      id: task.id,
      expression,
      command: task.run.toString(),
      description: task.description,
    })
  }
  // 保留原有实现
}
```

#### 验收标准

- [ ] 所有现有任务正确迁移到 Rust scheduler
- [ ] Cron 表达式正确转换
- [ ] 任务执行历史可查询
- [ ] 服务重启后任务状态恢复

---

## P1: 性能优化

### P1.1: 存储层迁移

**优先级**: 高
**复杂度**: 高
**预期收益**: 2-5x 性能提升

#### 现状问题

```typescript
// packages/ccode/src/storage/storage.ts (442 行)

// 问题 1: 每次 JSON 序列化开销
await Filesystem.atomicWrite(target, JSON.stringify(content, null, 2))

// 问题 2: 粗粒度锁
using _ = await Lock.write(target)

// 问题 3: 备份导致磁盘增长
await backup(key)  // 每次写入都创建备份
```

#### Rust 实现

```rust
// services/zero-storage/src/lib.rs (新建)

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct StorageService {
    db: Arc<Mutex<Connection>>,
    cache: Arc<RwLock<lru::LruCache<String, CachedValue>>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StoredValue<T> {
    pub key: String,
    pub value: T,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl<T> StorageService
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    /// 读取值（带缓存）
    pub async fn get(&self, key: &str) -> Result<Option<T>> {
        // 1. 检查缓存
        if let Some(cached) = self.cache.read().await.get(key) {
            return Ok(Some(cached.value.clone()));
        }

        // 2. 查询数据库
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare(
            "SELECT value, version FROM storage WHERE key = ?1"
        )?;

        let row = stmt.query_row(params![key], |row| {
            let value_json: String = row.get(0)?;
            let version: i64 = row.get(1)?;
            Ok((value_json, version))
        });

        match row {
            Ok((value_json, version)) => {
                let value: T = serde_json::from_str(&value_json)?;
                // 更新缓存
                self.cache.write().await.put(
                    key.to_string(),
                    CachedValue { value: value.clone(), version }
                );
                Ok(Some(value))
            }
            Err(e) if e == rusqlite::Error::QueryReturnedNoRows => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 写入值（原子、带版本控制）
    pub async fn set(&self, key: &str, value: &T) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let value_json = serde_json::to_string(value)?;

        let conn = self.db.lock().await;
        conn.execute(
            "INSERT INTO storage (key, value, version, created_at, updated_at)
             VALUES (?1, ?2, 1, ?3, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                version = version + 1,
                updated_at = ?3",
            params![key, value_json, now],
        )?;

        // 更新缓存
        self.cache.write().await.put(
            key.to_string(),
            CachedValue { value: value.clone(), version: 1 }
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_get() {
        let service = StorageService::in_memory();
        service.set("test", &42).await.unwrap();
        assert_eq!(service.get::<i32>("test").await.unwrap(), Some(42));
    }
}
```

#### TypeScript 适配层

```typescript
// packages/ccode/src/storage/rust-client.ts (新建)

export namespace Storage.Rust {
  const BASE_URL = "http://localhost:4440/api/storage"

  export async function read<T>(key: string[]): Promise<T> {
    const response = await fetch(`${BASE_URL}/${key.join("/")}`)
    if (!response.ok) {
      if (response.status === 404) {
        throw new NotFoundError({ message: `Key not found: ${key.join("/")}` })
      }
      throw new Error(`Storage error: ${response.statusText}`)
    }
    return response.json()
  }

  export async function write<T>(key: string[], content: T): Promise<void> {
    await fetch(`${BASE_URL}/${key.join("/")}`, {
      method: "PUT",
      body: JSON.stringify(content),
      headers: { "Content-Type": "application/json" },
    })
  }

  export async function update<T>(key: string[], fn: (draft: T) => void): Promise<T> {
    // 使用 Rust 端的事务更新
    const current = await read<T>(key)
    fn(current)
    await write(key, current)
    return current
  }
}
```

#### 数据迁移

```typescript
// packages/ccode/src/storage/migrate.ts (新建)

export async function migrateToRust(): Promise<void> {
  const keys = await list([])  // 获取所有现有键
  let migrated = 0

  for (const key of keys) {
    try {
      const value = await read(key)
      await Storage.Rust.write(key, value)
      migrated++
    } catch (e) {
      console.error(`Failed to migrate ${key.join("/")}:`, e)
    }
  }

  console.log(`Migrated ${migrated}/${keys.length} keys`)
}
```

#### 验收标准

- [ ] 所有存储操作正确迁移
- [ ] 性能基准：读取 < 1ms，写入 < 5ms
- [ ] 并发安全：多个进程同时访问无冲突
- [ ] 数据迁移 100% 成功

---

### P1.2: 追踪/日志系统迁移

**优先级**: 高
**复杂度**: 中
**预期收益**: 零拷贝序列化，与 Rust 服务统一追踪

#### 现状

```typescript
// packages/ccode/src/trace/query.ts (372 行)
// packages/ccode/src/trace/profiler.ts (512 行)
// packages/ccode/src/trace/storage.ts (316 行)
```

#### Rust 实现

```rust
// services/zero-tracing/src/lib.rs (新建)

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub ts: String,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service: String,
    pub event_type: String,
    pub level: String,
    pub payload: serde_json::Value,
}

pub struct TracingService {
    sender: mpsc::UnboundedSender<TraceEntry>,
}

impl TracingService {
    /// 接收追踪条目
    pub fn emit(&self, entry: TraceEntry) -> Result<()> {
        self.sender.send(entry)?;
        Ok(())
    }

    /// 查询追踪
    pub async fn query(&self, filter: TraceFilter) -> Result<Vec<TraceEntry>> {
        // 从 SQLite 或日志文件查询
    }
}

// HTTP 端点
pub fn tracing_http_routes() -> Router {
    Router::new()
        .route("/api/tracing/query", axum::routing::post(query_traces))
        .route("/api/tracing/services", axum::routing::get(list_services))
        .route("/api/tracing/errors", axum::routing::get(get_errors))
}
```

#### TypeScript 适配

```typescript
// packages/ccode/src/trace/rust-client.ts (新建)

export namespace Trace.Rust {
  export interface QueryFilter {
    service?: string
    trace_id?: string
    level?: string
    start_time?: string
    end_time?: string
  }

  export async function query(filter: QueryFilter): Promise<LogEntry[]> {
    const response = await fetch("http://localhost:4441/api/tracing/query", {
      method: "POST",
      body: JSON.stringify(filter),
    })
    return response.json()
  }

  export async function getServices(): Promise<string[]> {
    const response = await fetch("http://localhost:4441/api/tracing/services")
    return response.json()
  }
}
```

#### 验收标准

- [ ] 支持所有现有查询功能
- [ ] Rust 服务日志与 TS 日志统一格式
- [ ] 性能：查询 1000 条记录 < 100ms

---

## P2: 安全与计算

### P2.1: 权限系统迁移

**优先级**: 中
**复杂度**: 高
**预期收益**: 编译时安全，更高效的模式匹配

#### 现状

```typescript
// packages/ccode/src/permission/ (1,514 行)
// - 复杂的自动批准逻辑
// - 通配符匹配
// - 无持久化
```

#### Rust 实现

```rust
// services/zero-rbac/src/lib.rs (新建)

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResponse {
    Once,
    Always,
    Reject,
}

pub struct RbacService {
    /// 规则缓存: session_id -> { pattern -> response }
    rules: Arc<RwLock<HashMap<String, HashMap<String, PermissionResponse>>>>,

    /// 持久化存储
    store: Arc<RbacStore>,
}

impl RbacService {
    /// 检查权限
    pub fn check(
        &self,
        session_id: &str,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<PermissionResponse> {
        let rules = self.rules.read().unwrap();

        // 1. 精确匹配
        if let Some(rule) = rules.get(session_id).and_then(|r| r.get(tool)) {
            return Ok(rule.clone());
        }

        // 2. 通配符匹配
        for (pattern, response) in rules.get(session_id).iter().flatten() {
            if self.matches_pattern(pattern, tool) {
                return Ok(response.clone());
            }
        }

        // 3. 默认: 需要用户批准
        Ok(PermissionResponse::Once)
    }

    /// 模式匹配 (支持通配符)
    fn matches_pattern(&self, pattern: &str, tool: &str) -> bool {
        // 实现 glob 风格的模式匹配
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            return tool.starts_with(prefix);
        }
        pattern == tool
    }

    /// 设置权限规则
    pub fn set_rule(
        &self,
        session_id: &str,
        pattern: String,
        response: PermissionResponse,
    ) -> Result<()> {
        let mut rules = self.rules.write().unwrap();
        rules.entry(session_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(pattern, response);

        // 持久化
        self.store.save_rule(session_id, pattern, response)?;
        Ok(())
    }
}
```

#### 验收标准

- [ ] 所有现有权限规则正确迁移
- [ ] 通配符匹配 100% 兼容
- [ ] 权限状态持久化
- [ ] 性能：权限检查 < 1ms

---

### P2.2: 向量/嵌入系统迁移

**优先级**: 中
**复杂度**: 低
**预期收益**: 计算密集任务性能提升

#### 现有基础

`services/zero-memory` 已实现完整的向量/嵌入功能。

#### 迁移方案

直接使用 `zero-memory` 服务，无需额外开发。

```typescript
// packages/ccode/src/memory/rust-client.ts (新建)

export namespace Memory.Rust {
  const BASE_URL = "http://localhost:4443/api/memory"

  export async function search(query: string, limit = 10): Promise<MemoryResult[]> {
    const response = await fetch(`${BASE_URL}/search`, {
      method: "POST",
      body: JSON.stringify({ query, limit }),
    })
    return response.json()
  }

  export async function add(category: string, content: string): Promise<string> {
    const response = await fetch(`${BASE_URL}/add`, {
      method: "POST",
      body: JSON.stringify({ category, content }),
    })
    return response.json().then(r => r.id)
  }
}
```

---

## API 契约

### 统一响应格式

```typescript
// packages/ccode/src/rust-api/types.ts (新建)

/**
 * Rust 服务统一响应格式
 */
export interface RustApiResponse<T> {
  success: boolean
  data?: T
  error?: RustApiError
  meta?: {
    trace_id: string
    timestamp: string
  }
}

export interface RustApiError {
  code: string
  message: string
  details?: Record<string, unknown>
}

/**
 * 错误码定义
 */
export enum RustErrorCode {
  // 通用
  UNKNOWN = "UNKNOWN",
  INVALID_REQUEST = "INVALID_REQUEST",
  NOT_FOUND = "NOT_FOUND",

  // MCP
  MCP_CONNECTION_FAILED = "MCP_CONNECTION_FAILED",
  MCP_AUTH_REQUIRED = "MCP_AUTH_REQUIRED",

  // 存储层
  STORAGE_NOT_FOUND = "STORAGE_NOT_FOUND",
  STORAGE_CORRUPTED = "STORAGE_CORRUPTED",

  // 权限
  PERMISSION_DENIED = "PERMISSION_DENIED",
  PERMISSION_EXPIRED = "PERMISSION_EXPIRED",
}
```

### MCP API 契约

```typescript
// packages/ccode/src/rust-api/mcp.ts (新建)

export namespace RustApi.MCP {
  export interface ServerStatus {
    name: string
    status: "connected" | "disabled" | "failed" | "needs_auth"
    tools_count: number
    error?: string
  }

  export interface ToolDefinition {
    name: string
    description: string
    input_schema: Record<string, unknown>
  }

  export interface CallToolRequest {
    server: string
    tool: string
    arguments: Record<string, unknown>
  }

  export interface CallToolResponse {
    content: ToolContent[]
    is_error: boolean
  }

  export type ToolContent =
    | { type: "text"; text: string }
    | { type: "image"; data: string; mime_type: string }
}
```

### Scheduler API 契约

```typescript
// packages/ccode/src/rust-api/scheduler.ts (新建)

export namespace RustApi.Scheduler {
  export interface Task {
    id: string
    expression: string  // cron 表达式
    command: string
    description?: string
    next_run: string  // ISO 8601
    last_run?: string
    last_status?: string
  }

  export interface AddTaskRequest {
    id: string
    expression: string
    command: string
    description?: string
  }

  export interface TaskExecution {
    id: string
    task_id: string
    run_at: string
    status: "ok" | "error"
    output: string
  }
}
```

### Storage API 契约

```typescript
// packages/ccode/src/rust-api/storage.ts (新建)

export namespace RustApi.Storage {
  export interface GetRequest {
    key: string[]
  }

  export interface SetRequest {
    key: string[]
    value: unknown
    version?: number  // 乐观锁
  }

  export interface ListRequest {
    prefix: string[]
    limit?: number
  }

  export interface ListResponse {
    keys: string[][]
    total: number
  }

  export interface HealthReport {
    total: number
    healthy: number
    corrupted: Array<{ key: string[]; error: string }>
  }
}
```

### Tracing API 契约

```typescript
// packages/ccode/src/rust-api/tracing.ts (新建)

export namespace RustApi.Tracing {
  export interface LogEntry {
    ts: string
    trace_id: string
    span_id: string
    parent_span_id?: string
    service: string
    event_type: string
    level: string
    payload: Record<string, unknown>
  }

  export interface QueryRequest {
    service?: string
    trace_id?: string
    level?: string
    start_time?: string
    end_time?: string
    limit?: number
  }

  export interface QueryResponse {
    entries: LogEntry[]
    total: number
  }

  export interface ErrorGroup {
    key: string  // service:function:error_type
    count: number
    samples: Array<{
      error: string
      timestamp: string
      trace_id: string
    }>
  }
}
```

---

## 测试与验证

### 单元测试

每个迁移模块必须包含：

```rust
// Rust 端单元测试示例
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_set_get() {
        let service = StorageService::in_memory();
        service.set("test", &42).await.unwrap();
        assert_eq!(service.get::<i32>("test").await.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn test_storage_concurrent_access() {
        let service = Arc::new(StorageService::in_memory());
        let handles: Vec<_> = (0..100)
            .map(|i| {
                let service = Arc::clone(&service);
                tokio::spawn(async move {
                    service.set(&format!("key{}", i), &i).await.unwrap()
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        // 验证所有值正确写入
    }
}
```

```typescript
// TypeScript 端集成测试示例
describe("Storage.Rust", () => {
  it("should read and write values", async () => {
    await Storage.Rust.write(["test"], { value: 42 })
    const result = await Storage.Rust.read<{ value: number }>(["test"])
    expect(result.value).toBe(42)
  })

  it("should handle not found errors", async () => {
    await expect(
      Storage.Rust.read(["nonexistent"])
    ).rejects.toThrow(NotFoundError)
  })

  it("should fallback to TS implementation on error", async () => {
    // 模拟 Rust 服务不可用
    jest.spyOn(global, "fetch").mockRejectedValueOnce(new Error("Service unavailable"))

    // 应该降级到 TS 实现
    const result = await Storage.read(["test"])
    expect(result).toBeDefined()
  })
})
```

### 集成测试

```typescript
// packages/ccode/test/integration/rust-services.test.ts (新建)

describe("Rust Services Integration", () => {
  beforeAll(async () => {
    // 确保所有 Rust 服务已启动
    await ensureServicesRunning()
  })

  describe("MCP Service", () => {
    it("should list all servers", async () => {
      const status = await MCP.Rust.status()
      expect(Object.keys(status)).toBeDefined()
    })

    it("should connect to a local server", async () => {
      await MCP.Rust.connect("filesystem")
      const tools = await MCP.Rust.tools()
      expect(Object.keys(tools).length).toBeGreaterThan(0)
    })
  })

  describe("Storage Service", () => {
    it("should persist data across restarts", async () => {
      await Storage.Rust.write(["persistence", "test"], { value: true })

      // 重启服务 (通过 ops.sh)
      await exec("./ops.sh restart zero-storage")

      const result = await Storage.Rust.read<{ value: boolean }>(["persistence", "test"])
      expect(result.value).toBe(true)
    })
  })
})
```

### 性能基准

```typescript
// packages/ccode/test/benchmarks/rust-vs-ts.bench.ts (新建)

import { bench, describe } from "bun:bench"

describe("Storage: Rust vs TypeScript", () => {
  const testData = { key: "value", nested: { data: [1, 2, 3] } }

  bench("TypeScript storage write", async () => {
    await Storage.write(["bench", Date.now().toString()], testData)
  })

  bench("Rust storage write", async () => {
    await Storage.Rust.write(["bench", Date.now().toString()], testData)
  })

  bench("TypeScript storage read", async () => {
    await Storage.read(["bench", "test"])
  })

  bench("Rust storage read", async () => {
    await Storage.Rust.read(["bench", "test"])
  })
})

// 预期结果:
// - Rust 写入: ~2ms
// - TS 写入: ~10ms
// - Rust 读取: ~0.5ms (缓存命中)
// - TS 读取: ~5ms
```

---

## 回滚策略

### 功能开关

```typescript
// packages/ccode/src/config/features.ts (新建)

export namespace Features {
  export interface FeatureFlags {
    use_rust_mcp: boolean
    use_rust_scheduler: boolean
    use_rust_storage: boolean
    use_rust_tracing: boolean
    use_rust_rbac: boolean
  }

  let flags: FeatureFlags = {
    use_rust_mcp: true,
    use_rust_scheduler: true,
    use_rust_storage: false,  // 默认关闭，逐步启用
    use_rust_tracing: false,
    use_rust_rbac: false,
  }

  export function set(newFlags: Partial<FeatureFlags>) {
    flags = { ...flags, ...newFlags }
  }

  export function get(): FeatureFlags {
    return flags
  }

  export function isEnabled(flag: keyof FeatureFlags): boolean {
    return flags[flag] === true
  }
}
```

### 降级机制

```typescript
// packages/ccode/src/rust-api/client.ts (新建)

export class RustServiceClient {
  async request<T>(
    path: string,
    data?: unknown,
    options: { fallback?: () => Promise<T>; timeout?: number } = {}
  ): Promise<T> {
    const { fallback, timeout = 5000 } = options

    try {
      const controller = new AbortController()
      const timeoutId = setTimeout(() => controller.abort(), timeout)

      const response = await fetch(`${this.baseUrl}${path}`, {
        method: "POST",
        body: data ? JSON.stringify(data) : undefined,
        signal: controller.signal,
      })

      clearTimeout(timeoutId)

      if (!response.ok) {
        throw new RustApiError(`HTTP ${response.status}`)
      }

      const result = await response.json()
      if (!result.success) {
        throw new RustApiError(result.error?.message || "Unknown error")
      }

      return result.data
    } catch (e) {
      // Rust 服务不可用，尝试降级
      if (fallback) {
        console.warn(`Rust service unavailable, using fallback: ${path}`)
        return fallback()
      }
      throw e
    }
  }
}
```

### 数据回滚

```bash
#!/bin/bash
# scripts/rollback-storage.sh (新建)

set -euo pipefail

echo "Rolling back storage migration..."

# 1. 停止 Rust 服务
./ops.sh stop zero-storage

# 2. 从备份恢复数据
BACKUP_DIR="$HOME/.codecoder/storage/backup/$(date +%Y%m%d)"
if [ -d "$BACKUP_DIR" ]; then
  cp -r "$BACKUP_DIR"/* "$HOME/.codecoder/storage/"
  echo "Restored from $BACKUP_DIR"
fi

# 3. 禁用 Rust 存储
echo "Disabling Rust storage feature..."
# 更新配置文件

echo "Rollback complete. Please restart ccode."
```

---

## 实施时间表

### 第一阶段 (P0): 消除重复 - 预计 2 周

| 任务 | 时间 | 依赖 |
|------|------|------|
| MCP Rust HTTP API | 3 天 | - |
| MCP TS 适配层 | 2 天 | MCP API |
| MCP 集成测试 | 1 天 | 适配层 |
| Scheduler HTTP API | 2 天 | - |
| Scheduler TS 适配 | 1 天 | Scheduler API |
| Scheduler 集成测试 | 1 天 | 适配层 |
| P0 验收与文档 | 2 天 | 所有测试 |

### 第二阶段 (P1): 性能优化 - 预计 3 周

| 任务 | 时间 | 依赖 |
|------|------|------|
| Storage Rust 实现 | 5 天 | - |
| Storage 数据迁移 | 2 天 | Storage Rust |
| Storage TS 适配 | 2 天 | 数据迁移 |
| Storage 集成测试 | 2 天 | TS 适配 |
| Tracing Rust 实现 | 3 天 | - |
| Tracing TS 适配 | 2 天 | Tracing Rust |
| Tracing 集成测试 | 1 天 | TS 适配 |
| P1 验收与文档 | 2 天 | 所有测试 |

### 第三阶段 (P2): 安全与计算 - 预计 2 周

| 任务 | 时间 | 依赖 |
|------|------|------|
| RBAC Rust 实现 | 4 天 | - |
| RBAC TS 适配 | 2 天 | RBAC Rust |
| Vector 服务集成 | 1 天 | zero-memory |
| P2 验收与文档 | 2 天 | 所有测试 |

---

## 附录

### 文件清单

**需要新建的 TypeScript 文件**:

```
packages/ccode/src/
├── rust-api/
│   ├── client.ts        # 统一 Rust 服务客户端
│   ├── types.ts         # 类型定义
│   ├── mcp.ts           # MCP API 类型
│   ├── scheduler.ts     # Scheduler API 类型
│   ├── storage.ts       # Storage API 类型
│   └── tracing.ts       # Tracing API 类型
├── mcp/
│   └── rust-client.ts   # MCP Rust 适配层
├── scheduler/
│   └── rust-client.ts   # Scheduler Rust 适配层
├── storage/
│   ├── rust-client.ts   # Storage Rust 适配层
│   └── migrate.ts       # 数据迁移脚本
└── trace/
    └── rust-client.ts   # Tracing Rust 适配层
```

**需要新建的 Rust 文件**:

```
services/
├── zero-storage/        # 新建服务
│   └── src/
│       ├── lib.rs
│       ├── http.rs
│       └── cache.rs
├── zero-tracing/        # 新建服务
│   └── src/
│       ├── lib.rs
│       ├── query.rs
│       └── http.rs
├── zero-rbac/           # 新建服务
│   └── src/
│       ├── lib.rs
│       └── http.rs
├── zero-cli/src/mcp/
│   └── http.rs          # 新增 HTTP API
└── zero-workflow/src/
    └── http.rs          # 新增 HTTP API
```

### 参考文档

- [Axum HTTP 框架](https://docs.rs/axum/)
- [Tokio 异步运行时](https://tokio.rs/)
- [Rusqlite SQLite 库](https://docs.rs/rusqlite/)
- [Serde 序列化框架](https://serde.rs/)

---

**文档历史**:
- 2026-03-04: 初始版本创建
