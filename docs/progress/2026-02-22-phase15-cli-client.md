# Phase 15: CLI Client 迁移进展

## 基本信息

| 项目 | 内容 |
|------|------|
| 开始时间 | 2026-02-22 |
| 当前状态 | ✅ 完成 |
| 负责人 | AI Assistant |

---

## 完成内容

### 2026-02-22: providers 模块迁移

#### 变更概览

| 变更类型 | 文件 | 行数变化 |
|----------|------|----------|
| 新增 | `zero-gateway/src/provider/resilient.rs` | +310 |
| 修改 | `zero-gateway/src/provider/mod.rs` | +3 |
| 修改 | `zero-gateway/src/lib.rs` | +1 |
| 删除 | `zero-cli/src/providers/reliable.rs` | -239 |
| 修改 | `zero-cli/src/providers/mod.rs` | 重构 |

#### 架构决策

**问题**: zero-cli 有本地的 `ReliableProvider` 实现重试/回退逻辑，与 zero-gateway 存在代码重复风险。

**解决方案**:

1. 在 zero-gateway 中创建 `ResilientProvider`，提供可复用的重试/回退逻辑
2. zero-cli 使用 `GatewayProviderAdapter<ResilientProvider>` 桥接两个 Provider trait
3. 删除 zero-cli 本地的 `ReliableProvider`

**为什么保留两个 Provider trait**:

- `zero_gateway::Provider`: 完整 API，使用 `ChatRequest`/`ChatResponse`
- `zero_cli::Provider`: 简化 API，使用 `chat_with_system(system, message, model, temp)`

CLI 的简化 API 更适合命令行交互场景，避免每次调用都构建 `ChatRequest` 对象。

#### 代码实现

**zero-gateway ResilientProvider**:

```rust
pub struct ResilientProvider {
    providers: Vec<Arc<dyn Provider>>,
    config: ResilienceConfig,
}

#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    pub max_retries: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}
```

功能特点:
- 指数退避重试 (base * 2^attempt)
- 多 provider 链式回退
- 聚合错误报告
- 完整的单元测试覆盖

**zero-cli create_resilient_provider**:

```rust
pub fn create_resilient_provider(
    primary_name: &str,
    api_key: Option<&str>,
    reliability: &crate::config::ReliabilityConfig,
) -> anyhow::Result<Box<dyn Provider>> {
    // 1. 创建 gateway providers
    let gateway_providers: Vec<Arc<dyn zero_gateway::Provider>> = ...;

    // 2. 包装为 ResilientProvider
    let resilient = ResilientProvider::new(gateway_providers, resilience_config);

    // 3. 适配为 CLI Provider trait
    Ok(Box::new(GatewayProviderAdapter::new(resilient)))
}
```

#### 测试结果

```
zero-gateway: 103 tests passed (新增 7 个 ResilientProvider 测试)
zero-cli providers: 25 tests passed
```

---

### 2026-02-22: memory 模块迁移

#### 变更概览

| 变更类型 | 文件 | 行数变化 |
|----------|------|----------|
| 修改 | `zero-memory/src/sqlite.rs` | +35 (新增 prune 方法和测试) |
| 修改 | `zero-cli/src/memory/hygiene.rs` | 重构 |

#### 架构决策

**问题**: `hygiene.rs` 直接使用 `rusqlite` 操作数据库删除旧对话记录

**解决方案**:

1. 在 `zero-memory/SqliteMemory` 添加 `prune_category_older_than()` 方法
2. 更新 `zero-cli/hygiene.rs` 使用该方法替代直接数据库操作
3. 保留 `hygiene.rs` 在 zero-cli（文件归档是 workspace 特定功能）

**为什么 hygiene.rs 不移到 zero-memory**:

- 文件归档逻辑与 CLI workspace 结构紧耦合
- Session 文件管理是 CLI 特有功能
- 只有 SQLite 清理逻辑适合共享

#### 新增 API

**`SqliteMemory::prune_category_older_than()`**:

```rust
/// Prune entries in a specific category older than the given cutoff.
pub async fn prune_category_older_than(
    &self,
    category: MemoryCategory,
    cutoff_rfc3339: &str,
) -> anyhow::Result<u64>
```

**`SqliteMemory::db_path()`**:

```rust
/// Get the path to the underlying database file.
pub fn db_path(&self) -> &Path
```

#### 测试结果

```
zero-memory sqlite: 20 tests passed (新增 3 个测试)
zero-cli memory: 9 tests passed
```

---

## 已完成阶段

### 第三阶段: agent 模块迁移 ✅ 完成

**分析结果**:

| 组件 | 位置 | 职责 |
|------|------|------|
| `loop_.rs` | zero-cli | CLI 应用层 - 配置、内存、交互模式 |
| `AgentExecutor` | zero-agent | 核心执行 - 工具调用循环 |

**变更内容**:

1. 扩展 `zero_cli::Provider` trait 添加 `name()` 和 `supports_model()`
2. `GatewayProviderAdapter` 同时实现两个 Provider trait:
   - `zero_cli::providers::Provider` (CLI 使用)
   - `zero_agent::Provider` (AgentExecutor 使用)

**代码示例**:

```rust
// GatewayProviderAdapter 现在可以用于 AgentExecutor
let provider: Arc<dyn zero_agent::Provider> = Arc::new(
    GatewayProviderAdapter::new(ResilientProvider::new(...))
);

let executor = AgentExecutor::new(
    provider,
    tools,
    system_prompt,
    model,
    temperature,
);
```

**测试结果**:

```
zero-cli providers: 25 tests passed
```

---

### 第四阶段: tools 模块迁移 ✅ 完成（无需迁移）

**分析结果**:

经过详细分析，zero-cli 中的 tools 模块已经是正确的架构，**无需迁移**：

| 工具 | 位置 | 决策理由 |
|------|------|----------|
| `registry.rs` | 保留 zero-cli | 应用层编排器，管理 native + MCP 工具，依赖 CLI 特定配置 |
| `skill_search.rs` | 保留 zero-cli | 依赖 CLI 的 SkillHub 技能管理系统 |
| `browser_open.rs` | 保留 zero-cli | 系统浏览器交互，CLI 专有 |
| `auto_login.rs` | 保留 zero-cli | CLI 认证功能 |

**架构验证**:

核心可复用工具已在 zero-tools 中：
- `shell.rs`, `browser.rs`, `file_read.rs`, `file_write.rs`
- `memory_recall.rs`, `memory_store.rs`, `memory_forget.rs`
- `codecoder.rs`, `security.rs`

zero-cli 通过 re-export 机制使用 zero-tools：

```rust
// zero-cli/src/tools/mod.rs
pub use zero_tools::{Tool, ToolResult, ToolSpec};
pub use zero_tools::{ShellTool, FileReadTool, FileWriteTool, ...};
```

**ToolRegistry 设计**:

```rust
pub struct ToolRegistry {
    /// Native ZeroBot tools (from zero-tools)
    native_tools: Vec<Arc<dyn Tool>>,
    /// MCP tools from external servers (CLI-specific)
    mcp_tools: RwLock<Vec<Arc<McpToolAdapter>>>,
    /// MCP manager for connection lifecycle
    mcp_manager: RwLock<McpManager>,
}
```

这是典型的应用层编排模式：
1. 核心工具实现在库层（zero-tools）
2. 工具注册和 MCP 集成在应用层（zero-cli）
3. 保持关注点分离

---

## 总结

### Phase 15 完成状态

| 阶段 | 状态 | 变更类型 |
|------|------|----------|
| 第一阶段: providers 迁移 | ✅ 完成 | 创建 ResilientProvider，删除 ReliableProvider |
| 第二阶段: memory 迁移 | ✅ 完成 | 添加 prune_category_older_than 方法 |
| 第三阶段: agent 迁移 | ✅ 完成 | 扩展 Provider trait，适配器模式 |
| 第四阶段: tools 迁移 | ✅ 完成 | 无需迁移，架构已正确 |

### 架构成果

```
┌─────────────────────────────────────────────────────────────┐
│                       zero-cli (应用层)                      │
├─────────────────────────────────────────────────────────────┤
│  providers/mod.rs     → GatewayProviderAdapter              │
│  memory/hygiene.rs    → 使用 zero-memory::SqliteMemory      │
│  agent/loop_.rs       → 使用 zero-agent::AgentExecutor      │
│  tools/registry.rs    → 编排 zero-tools + MCP              │
└──────────────────────────┬──────────────────────────────────┘
                           │ 依赖
┌──────────────────────────▼──────────────────────────────────┐
│                      库层 (可复用)                           │
├─────────────────────────────────────────────────────────────┤
│  zero-gateway  → ResilientProvider (重试/回退)              │
│  zero-memory   → SqliteMemory (包含 prune 方法)             │
│  zero-agent    → AgentExecutor (工具调用循环)               │
│  zero-tools    → ShellTool, FileReadTool, MemoryTool...    │
└─────────────────────────────────────────────────────────────┘
```

### 代码变化统计

| 包 | 新增行 | 删除行 | 净变化 |
|----|--------|--------|--------|
| zero-gateway | +310 | 0 | +310 |
| zero-memory | +35 | 0 | +35 |
| zero-cli | +50 | -239 | -189 |

**总净变化**: 代码减少 ~189 行，同时增加了可复用性

---

## 关键文件

| 文件路径 | 说明 |
|----------|------|
| `services/zero-gateway/src/provider/resilient.rs` | 新增的弹性 Provider 包装器 |
| `services/zero-gateway/src/lib.rs` | 导出 ResilientProvider |
| `services/zero-cli/src/providers/mod.rs` | 重构后的 providers 模块 |
| `services/zero-memory/src/sqlite.rs` | 新增 prune_category_older_than 方法 |
| `services/zero-cli/src/memory/hygiene.rs` | 使用 zero-memory 的 prune 方法 |

---

## 验证命令

```bash
# 构建检查
cd services && cargo build -p zero-gateway -p zero-memory -p zero-cli

# 运行测试
cd services && cargo test -p zero-gateway --lib
cd services && cargo test -p zero-memory --lib sqlite::
cd services && cargo test -p zero-cli --lib providers::
cd services && cargo test -p zero-cli --lib memory::

# Clippy 检查
cd services && cargo clippy -p zero-gateway -p zero-memory -p zero-cli
```
