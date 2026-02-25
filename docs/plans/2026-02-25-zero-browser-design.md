# zero-browser 服务设计文档

> **状态**: 设计完成，待实现
> **日期**: 2026-02-25
> **作者**: Claude + User

## 概述

zero-browser 是一个 Rust 实现的浏览器自动化服务，核心功能是 **API 学习模式**：通过 CDP (Chrome DevTools Protocol) 监听网络流量，自动提取 API 模式，并支持无浏览器重放。

### 目标

1. 监听浏览器网络请求，学习 API 端点模式
2. 提取认证方式、请求结构、响应格式
3. 存储学习结果到 zero-memory
4. 支持 API 重放（无需浏览器）

### 非目标

- WebSocket 流量学习（v1 不支持）
- 前端 JS 分析（独立功能）
- 自动找 key / 撸额度（不建议实现）

---

## 架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                        zero-browser :4433                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   Browser    │    │   Network    │    │   Pattern    │      │
│  │  Controller  │───▶│   Monitor    │───▶│   Learner    │      │
│  │              │    │              │    │              │      │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘      │
│         │                   │                   │               │
│         │ CDP WebSocket     │ Intercept         │ Store         │
│         ▼                   ▼                   ▼               │
│  ┌──────────────────────────────────────────────────────┐      │
│  │                Chrome/Chromium (Headless)             │      │
│  └──────────────────────────────────────────────────────┘      │
│         │                                                       │
│         ├───────────────────────────────────────┐               │
│         ▼                                       ▼               │
│  ┌──────────────┐                        ┌──────────────┐      │
│  │ zero-memory  │                        │ API Replay   │      │
│  │  (patterns)  │                        │   Engine     │      │
│  └──────────────┘                        └──────────────┘      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 核心组件

| 组件 | 职责 |
|------|------|
| Browser Controller | 管理浏览器生命周期，执行 DOM 操作 |
| Network Monitor | 通过 CDP 拦截所有网络请求/响应 |
| Pattern Learner | 从原始流量中提取可重用的 API 模式 |
| API Replay Engine | 无需浏览器，直接调用学习到的 API |

### 技术选型

| 组件 | 技术 | 理由 |
|------|------|------|
| CDP 客户端 | chromiumoxide | 纯 Rust，async/await |
| HTTP 框架 | axum 0.7 | 与其他 zero-* 服务一致 |
| 存储 | zero-memory | 直接库调用，无序列化开销 |
| 配置 | zero-common | 统一配置格式 |
| 日志 | tracing | 结构化日志 |

---

## 数据结构

### ApiPattern - 学习到的 API 模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPattern {
    /// 唯一标识: {host}:{method}:{path_pattern}
    pub id: String,
    /// 来源域名
    pub host: String,
    /// HTTP 方法
    pub method: String,
    /// 路径模式 (支持参数提取: /users/{id})
    pub path_pattern: String,
    /// 必需的请求头
    pub required_headers: HashMap<String, HeaderPattern>,
    /// 认证方式
    pub auth: Option<AuthPattern>,
    /// 请求体模式 (JSON Schema)
    pub request_schema: Option<serde_json::Value>,
    /// 响应体模式
    pub response_schema: Option<serde_json::Value>,
    /// 学习时间
    pub learned_at: DateTime<Utc>,
    /// 使用次数
    pub usage_count: u32,
    /// 最后成功时间
    pub last_success: Option<DateTime<Utc>>,
}
```

### HeaderPattern - 请求头模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeaderPattern {
    /// 固定值
    Fixed(String),
    /// 动态值 (需要从上下文提取)
    Dynamic { source: String, key: String },
    /// 从认证获取
    FromAuth,
}
```

### AuthPattern - 认证模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthPattern {
    /// Bearer Token
    Bearer { token_source: String },
    /// Cookie 认证
    Cookie { names: Vec<String> },
    /// API Key
    ApiKey { header: String, key_source: String },
    /// OAuth2
    OAuth2 {
        token_url: String,
        client_id_source: String,
        client_secret_source: String,
    },
}
```

---

## HTTP API

### 浏览器控制

```yaml
POST /browser/sessions
  # 创建新浏览器会话
  Request: { headless: bool, proxy?: str, viewport: {width, height} }
  Response: { session_id: str }

DELETE /browser/sessions/{session_id}
  # 关闭浏览器会话

POST /browser/sessions/{session_id}/navigate
  # 导航到 URL
  Request: { url: str, wait_until: "load" | "domcontentloaded" | "networkidle" }

POST /browser/sessions/{session_id}/action
  # 执行 DOM 操作
  Request: { action: str, selector: str, value?: str }

GET /browser/sessions/{session_id}/snapshot
  # 获取可访问性快照
```

### 网络学习

```yaml
POST /browser/sessions/{session_id}/learn/start
  # 开始网络学习模式
  Request: {
    filter: {
      hosts: [str],        # 只学习这些域名的 API
      path_prefix?: str,   # 路径前缀过滤
      methods?: [str]      # 仅特定方法
    }
  }

POST /browser/sessions/{session_id}/learn/stop
  # 停止学习，返回提取的模式
  Response: {
    patterns: [ApiPattern],
    request_count: int,
    unique_endpoints: int
  }

GET /browser/sessions/{session_id}/network/requests
  # 获取当前会话的所有网络请求
```

### 模式管理

```yaml
GET /patterns
  # 列出所有学习到的 API 模式
  Query: { host?: str, method?: str }

GET /patterns/{pattern_id}
  # 获取单个模式详情

DELETE /patterns/{pattern_id}
  # 删除模式
```

### API 重放

```yaml
POST /replay
  # 重放学习到的 API（无需浏览器）
  Request: {
    pattern_id: str,
    path_params?: {key: value},
    query_params?: {key: value},
    body?: json,
    auth?: { type: "bearer" | "cookie" | "apikey", value: str }
  }
  Response: {
    status: int,
    headers: {key: value},
    body: json,
    duration_ms: int
  }
```

---

## 典型工作流

```
1. 创建会话    POST /browser/sessions
2. 导航       POST /browser/sessions/{id}/navigate
3. 开始学习   POST /browser/sessions/{id}/learn/start
4. 用户操作   POST /browser/sessions/{id}/action (多次)
5. 停止学习   POST /browser/sessions/{id}/learn/stop → 获得 patterns
6. 关闭会话   DELETE /browser/sessions/{id}
7. 重放 API   POST /replay (无需浏览器，直接 HTTP 调用)
```

---

## 错误处理

### 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Browser launch failed: {0}")]
    LaunchFailed(String),

    #[error("Navigation timeout: {url}")]
    NavigationTimeout { url: String },

    #[error("Element not found: {selector}")]
    ElementNotFound { selector: String },

    #[error("Network interception failed: {0}")]
    NetworkError(String),

    #[error("Pattern extraction failed: {reason}")]
    PatternExtractionFailed { reason: String },

    #[error("Replay failed: {pattern_id} - {reason}")]
    ReplayFailed { pattern_id: String, reason: String },

    #[error("Memory storage failed: {0}")]
    MemoryError(#[from] zero_memory::Error),
}
```

### 边界情况处理

| 场景 | 处理策略 |
|------|---------|
| Chrome 未安装 | 启动时检测，返回安装指引 |
| 会话超时 | 30 分钟无活动自动清理 |
| 网络请求过大 | 限制 body 大小 (10MB)，超出仅记录元数据 |
| CORS 请求 | 自动合并 preflight + 实际请求 |
| WebSocket 流量 | v1 暂不支持，记录警告 |
| 动态 Token | 检测 `Authorization` 头变化，标记为 `Dynamic` |
| 分页 API | 检测相同路径不同参数，提取分页模式 |
| GraphQL | 特殊处理：按 operationName 区分，提取 query 结构 |

---

## 文件结构

```
services/zero-browser/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口
│   ├── lib.rs               # 库导出
│   ├── browser/
│   │   ├── mod.rs
│   │   ├── controller.rs    # 浏览器控制
│   │   └── session.rs       # 会话管理
│   ├── network/
│   │   ├── mod.rs
│   │   ├── monitor.rs       # 网络监听
│   │   └── interceptor.rs   # 请求拦截
│   ├── pattern/
│   │   ├── mod.rs
│   │   ├── extractor.rs     # 模式提取
│   │   └── types.rs         # 数据类型
│   ├── replay/
│   │   ├── mod.rs
│   │   └── executor.rs      # API 重放
│   └── routes.rs            # HTTP 路由
└── tests/
    ├── browser_integration.rs
    └── replay_test.rs
```

---

## 测试策略

- **单元测试**: 80%+ 代码覆盖
- **集成测试**: 核心流程 (创建 → 学习 → 重放)
- **E2E 测试**: 真实网站登录场景（需要配置凭据）

---

## 服务配置

```json
{
  "browser": {
    "host": "127.0.0.1",
    "port": 4433,
    "chrome_path": null,
    "headless": true,
    "session_timeout_secs": 1800,
    "max_body_size_mb": 10
  }
}
```

---

## 依赖

```toml
[dependencies]
chromiumoxide = "0.5"
tokio = { version = "1.0", features = ["full"] }
axum = "0.7"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
zero-common = { path = "../zero-common" }
zero-memory = { path = "../zero-memory" }
reqwest = { version = "0.11", features = ["json"] }
```

---

## 后续扩展

1. **v1.1**: WebSocket 流量学习
2. **v1.2**: GraphQL introspection 自动提取
3. **v1.3**: 与 zero-workflow 集成，支持 `browser` step type
4. **v2.0**: 前端 JS 分析（独立功能）
