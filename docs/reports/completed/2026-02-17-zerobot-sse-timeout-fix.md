# ZeroBot CodeCoder SSE 连接和任务超时问题修复

**完成日期**: 2026-02-17 (更新: 2026-02-18)
**状态**: 已完成

## 问题背景

用户通过 Telegram 使用 ZeroBot 查询航班信息时遇到以下问题：

1. **SSE 连接失败**: `error sending request for url (http://localhost:4400/api/v1/tasks/.../events)`
2. **任务卡在 "running" 状态**: 第二个任务轮询 27+ 次仍未完成

## 根因分析

### 问题 1: 端口配置不匹配

- `codecoder.rs` 默认端口是 4096
- `ops.sh` 运维脚本实际使用端口 4400
- 导致 ZeroBot 连接错误的端口

### 问题 2: SSE 连接无重试

- 当 SSE 连接失败时，代码直接回退到轮询
- 没有重试机制来处理短暂的网络问题或服务器负载

### 问题 3: 任务执行无超时

- `executeTask()` 中的 `LocalSession.prompt()` 可能无限等待
- 当 agent 执行长时间操作或等待权限时，任务会卡住

### 问题 4: reqwest 超时配置不当 (2026-02-18 发现)

- 使用 `.timeout()` 设置整体请求超时对 SSE 长连接不适用
- 缺少 `.connect_timeout()` 导致连接建立时等待过长
- IPv6 连接失败后切换到 IPv4 的时间不足

### 问题 5: IPv6/IPv4 地址解析问题 (2026-02-18 发现) - 根本原因

**这是 SSE 连接持续失败的根本原因**

**现象分析：**
```
SSE connection error (attempt 1/3): error sending request for url
(http://localhost:4400/api/v1/tasks/.../events)
[connect=false, timeout=false, source=Some("client error (SendRequest)")]
```

- `connect=false` 表示不是连接超时
- `timeout=false` 表示不是整体超时
- `source=Some("client error (SendRequest)")` 表示请求发送阶段失败

**根本原因：**

1. CodeCoder API 服务器默认监听 `127.0.0.1:4400`（仅 IPv4）
2. ZeroBot 客户端连接 `localhost:4400`
3. 系统 DNS 解析 `localhost` 时优先返回 IPv6 地址 `::1`
4. reqwest 尝试连接 `[::1]:4400`，但服务器不监听 IPv6，连接被拒绝
5. reqwest 可能没有正确实现 Happy Eyeballs 算法回退到 IPv4

**验证：**
```bash
$ curl -v http://localhost:4400/
* Host localhost:4400 was resolved.
* IPv6: ::1
* IPv4: 127.0.0.1
*   Trying [::1]:4400...
* connect to ::1 port 4400 from ::1 port 65176 failed: Connection refused
*   Trying 127.0.0.1:4400...
* Connected to localhost (127.0.0.1) port 4400
```

curl 能正确回退到 IPv4，但 reqwest 客户端没有正确处理这种情况。

## 完成的修改

### 1. 修复端口配置

#### `services/zero-bot/src/tools/codecoder.rs`
```rust
// 修改前
const DEFAULT_ENDPOINT: &str = "http://localhost:4096";

// 修改后
const DEFAULT_ENDPOINT: &str = "http://localhost:4400";
```

#### `services/zero-bot/src/config/schema.rs`
```rust
fn default_codecoder_endpoint() -> String {
    "http://localhost:4400".into()  // 从 4096 改为 4400
}
```

### 2. 添加 SSE 连接重试逻辑

#### `services/zero-bot/src/tools/codecoder.rs`

新增常量：
```rust
/// SSE connection retry attempts
const SSE_MAX_RETRIES: u32 = 3;
/// SSE connection timeout in seconds (time to establish connection)
const SSE_CONNECT_TIMEOUT_SECS: u64 = 30;
/// Delay between SSE retry attempts in seconds
const SSE_RETRY_DELAY_SECS: u64 = 2;
```

修改 `stream_task_events()` 函数：
- 添加重试循环，最多重试 3 次
- 每次重试间隔递增（2s, 4s, 6s）
- 详细日志记录每次尝试和失败原因
- 所有重试失败后才回退到轮询

### 3. 优化 reqwest SSE 客户端配置 (2026-02-18)

```rust
// 修改前 - 不适合 SSE 长连接
let sse_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(SSE_TIMEOUT_SECS))  // 整体超时会导致 SSE 断开
    .build()
    .unwrap_or_default();

// 修改后 - 优化 SSE 连接
let sse_client = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(SSE_CONNECT_TIMEOUT_SECS))  // 仅连接超时
    .tcp_nodelay(true)  // 减少事件流延迟
    .build()
    .unwrap_or_default();
```

关键改进：
- 移除 `.timeout()` - SSE 是长连接，不应有整体超时
- 添加 `.connect_timeout(30s)` - 给足时间处理 IPv6→IPv4 回退
- 添加 `.tcp_nodelay(true)` - 减少 SSE 事件流延迟

### 4. 增强错误日志 (2026-02-18)

```rust
Err(e) => {
    // 记录详细错误信息用于调试
    let is_connect = e.is_connect();
    let is_timeout = e.is_timeout();
    let is_request = e.is_request();
    let source = e.source().map(|s| s.to_string());

    tracing::warn!(
        "SSE connection error (attempt {}/{}): {} [connect={}, timeout={}, source={:?}]",
        attempt, SSE_MAX_RETRIES, e, is_connect, is_timeout, source
    );
}
```

### 5. 添加任务执行超时机制

#### `packages/ccode/src/api/server/handlers/task.ts`

新增常量：
```typescript
/** Task execution timeout in milliseconds (5 minutes) */
const TASK_TIMEOUT_MS = 5 * 60 * 1000
```

修改 `executeTask()` 函数：
```typescript
// 创建超时 Promise
const timeoutPromise = new Promise<never>((_, reject) => {
  setTimeout(() => {
    reject(new Error(`Task execution timeout after ${TASK_TIMEOUT_MS / 1000} seconds`))
  }, TASK_TIMEOUT_MS)
})

// 使用 Promise.race 限制执行时间
await Promise.race([
  LocalSession.prompt({...}),
  timeoutPromise,
])
```

### 6. 增强 TypeScript 日志输出

#### `packages/ccode/src/api/server/handlers/task.ts`

添加结构化日志：
- 任务开始时记录：taskID, agent, sessionID, userID, platform, promptLength
- 权限请求时记录：taskID, requestID, permission, patterns
- 任务超时时记录：taskID, agent, timeoutMs, elapsedMs
- 任务完成时记录：taskID, agent, elapsedMs, outputLength
- 任务失败时记录：taskID, agent, elapsedMs, isTimeout, error

### 7. 修复 IPv6/IPv4 地址解析问题 (2026-02-18) - 关键修复

**客户端修复 - 使用 IPv4 地址代替 localhost：**

#### `services/zero-bot/src/tools/codecoder.rs`
```rust
// 修改前
const DEFAULT_ENDPOINT: &str = "http://localhost:4400";

// 修改后 - 避免 IPv6 解析问题
/// Default `CodeCoder` API endpoint
/// Use 127.0.0.1 instead of localhost to avoid IPv6 resolution issues
const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4400";
```

#### `services/zero-bot/src/config/schema.rs`
```rust
fn default_codecoder_endpoint() -> String {
    // Use 127.0.0.1 instead of localhost to avoid IPv6 resolution issues.
    // When using 'localhost', the system may first try IPv6 (::1) which fails
    // if the server only listens on IPv4, causing SSE connection failures.
    "http://127.0.0.1:4400".into()
}
```

**服务器端修复 - 同时监听 IPv4 和 IPv6：**

#### `packages/ccode/src/api/server/index.ts`
```typescript
// 修改前
const hostname = options.hostname ?? "127.0.0.1"

// 修改后 - 监听所有接口（IPv4 和 IPv6）
// Use "::" to listen on both IPv4 and IPv6, avoiding connection issues
// when clients resolve "localhost" to IPv6 (::1) first
const hostname = options.hostname ?? "::"
```

## 验证结果

- ✅ Rust 编译成功（无警告）
- ✅ TypeScript 类型检查通过（预先存在的 provider.ts 错误不影响）
- ✅ curl 测试 SSE 端点成功
- ✅ ZeroBot release 构建成功
- ⏳ 等待服务重启后实际测试验证

## 配置说明

### 超时配置

| 配置项 | 值 | 说明 |
|--------|-----|------|
| TASK_TIMEOUT_MS | 5 分钟 | 任务整体超时 |
| SSE_CONNECT_TIMEOUT_SECS | 30 秒 | SSE 连接建立超时 |
| SSE_CHUNK_TIMEOUT_SECS | 2 分钟 | SSE 单次数据块超时 |
| SSE_MAX_RETRIES | 3 次 | SSE 连接重试次数 |
| SSE_RETRY_DELAY_SECS | 2 秒 | SSE 重试基础延迟 |

### 端口配置

| 服务 | 端口 |
|------|------|
| CodeCoder API Server | 4400 |
| Web Frontend | 4401 |
| ZeroBot Daemon | 4402 |

### 地址配置

| 配置 | 修改前 | 修改后 | 原因 |
|------|--------|--------|------|
| 客户端默认端点 | `localhost:4400` | `127.0.0.1:4400` | 避免 IPv6 解析 |
| 服务器监听地址 | `127.0.0.1` | `::` | 同时支持 IPv4/IPv6 |

## 重启服务

修改生效需要重启以下服务：

```bash
# 1. 重启 CodeCoder API 服务器
# 停止当前服务
pkill -f "bun.*serve"

# 启动新服务（会使用新的监听地址 ::）
cd /path/to/project && bun dev serve --port 4400

# 2. 重启 ZeroBot（使用新构建的 release 版本）
pkill -f "zero-bot"
./target/release/zero-bot daemon
```

## 临时解决方案

如果 SSE 连接仍然失败，系统会自动回退到轮询模式（每 2 秒轮询一次，最多 6 分钟）。

## 相关文档

- [权限系统实现报告](./2026-02-17-zerobot-codecoder-permission.md)
