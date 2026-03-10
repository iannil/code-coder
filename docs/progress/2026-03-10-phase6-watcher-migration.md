# Phase 6: 四大 Watcher 迁移到 Rust

> 日期: 2026-03-10
> 状态: ✅ 完成
> 时间: 23:45:00

## 概述

将四大 Watcher 从 TypeScript 迁移到 Rust，作为 Observer Network 迁移的最后一步。

## 实施内容

### 1. Watcher Trait 和基础设施

**文件**: `services/zero-cli/src/observer/watchers/mod.rs`, `base.rs`

```rust
#[async_trait]
pub trait Watcher: Send + Sync {
    fn id(&self) -> &str;
    fn watcher_type(&self) -> WatcherType;
    fn is_running(&self) -> bool;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn observe(&mut self) -> Option<Observation>;
    fn get_status(&self) -> WatcherStatus;
    fn get_metrics(&self) -> WatcherMetrics;
}
```

**BaseWatcherState** 提供:
- 延迟追踪 (latency recording)
- 错误率计算 (error rate)
- 健康状态 (Healthy/Degraded/Failing/Stopped)
- Glob 过滤器匹配

### 2. CodeWatch

**文件**: `services/zero-cli/src/observer/watchers/code_watch.rs` (~720 行)

**功能**:
- Git 变更检测 (使用 `git2` crate 原生 Git 操作)
- TypeScript 类型检查 (调用 `bun turbo typecheck`)
- 类型错误解析 (regex pattern matching)
- 影响范围分析 (file/package/module/project)

**配置**:
```rust
pub struct CodeWatchConfig {
    pub watch_paths: Vec<PathBuf>,
    pub git_root: Option<PathBuf>,
    pub track_build: bool,
    pub track_tests: bool,
    pub enable_typecheck: bool,
    pub typecheck_interval_ms: u64,
}
```

### 3. WorldWatch

**文件**: `services/zero-cli/src/observer/watchers/world_watch.rs` (~400 行)

**功能**:
- 市场数据观察 (MarketDataPoint: symbol, price, change, volume)
- 新闻相关性计算 (keyword matching: ai, rust, crypto, etc.)
- 安全公告监控 (SecurityAdvisory: cve_id, severity, affected_packages)
- API 变更检测

**配置**:
```rust
pub struct WorldWatchConfig {
    pub track_market: bool,
    pub track_news: bool,
    pub track_security: bool,
    pub news_relevance_threshold: f32,
    pub market_symbols: Vec<String>,
}
```

### 4. SelfWatch

**文件**: `services/zero-cli/src/observer/watchers/self_watch.rs` (~600 行)

**功能**:
- Agent 行为跟踪 (AgentActionRecord: agent_name, action, success, duration)
- 资源使用快照 (ResourceSnapshot: tokens, cost, api_calls)
- 成本峰值检测 (SessionCostRecord: cost_spike detection)
- 错误模式识别 (滑动窗口分析, consecutive error detection)

**核心类型**:
```rust
pub struct AgentActionRecord {
    pub agent_name: String,
    pub action: String,
    pub success: bool,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

pub struct ResourceSnapshot {
    pub total_tokens: u64,
    pub total_cost: f64,
    pub api_calls: u64,
    pub timestamp: DateTime<Utc>,
}
```

### 5. MetaWatch

**文件**: `services/zero-cli/src/observer/watchers/meta_watch.rs` (~550 行)

**功能**:
- 观察者健康监控 (WatcherHealthRecord per watcher)
- 覆盖率分析 (area coverage: code/world/self/meta)
- 共识漂移检测 (confidence variance tracking)
- 系统健康报告 (calibrate() 主动检查)

**核心类型**:
```rust
pub struct SystemHealth {
    pub overall: HealthStatus,
    pub watchers: HashMap<String, HealthStatus>,
    pub coverage_score: f32,
    pub consensus_drift: f32,
    pub latency_status: LatencyStatus,
}

pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Critical { reason: String },
}
```

### 6. API 端点

**文件**: `services/zero-cli/src/unified_api/observer.rs`

```
GET  /api/v1/observer/watchers           # 列出所有 watcher
GET  /api/v1/observer/watchers/:id       # 获取 watcher 详情 + 指标
POST /api/v1/observer/watchers/:id/start # 启动 watcher
POST /api/v1/observer/watchers/:id/stop  # 停止 watcher
```

**响应类型**:
```rust
pub struct WatcherListResponse {
    pub watchers: Vec<WatcherStatus>,
    pub total: usize,
}

pub struct WatcherDetailResponse {
    pub status: WatcherStatus,
    pub metrics: WatcherMetrics,
}
```

## 依赖变更

**Cargo.toml**:
```toml
# Observer watchers
git2 = "0.19"  # 原 0.18，改为匹配 zero-core
notify = { version = "6.1", default-features = false, features = ["macos_fsevent"] }
```

**版本冲突修复**:
- `git2 0.18` 依赖 `libgit2-sys 0.16`
- `zero-core` 使用 `git2 0.19` (依赖 `libgit2-sys 0.17`)
- 统一使用 `git2 0.19` 解决 native library linking 冲突

## 测试结果

```bash
running 37 tests
test observer::watchers::base::tests::test_watcher_state_creation ... ok
test observer::watchers::base::tests::test_health_calculation ... ok
test observer::watchers::base::tests::test_filter_matching ... ok
test observer::watchers::base::tests::test_generate_id ... ok
test observer::watchers::code_watch::tests::test_code_watch_creation ... ok
test observer::watchers::code_watch::tests::test_parse_type_errors ... ok
test observer::watchers::code_watch::tests::test_determine_scope ... ok
test observer::watchers::code_watch::tests::test_code_watch_config_defaults ... ok
test observer::watchers::world_watch::tests::test_world_watch_creation ... ok
test observer::watchers::world_watch::tests::test_calculate_news_relevance ... ok
test observer::watchers::world_watch::tests::test_market_data_observation ... ok
test observer::watchers::world_watch::tests::test_security_advisory_observation ... ok
test observer::watchers::self_watch::tests::test_self_watch_creation ... ok
test observer::watchers::self_watch::tests::test_error_pattern_detection ... ok
test observer::watchers::meta_watch::tests::test_meta_watch_creation ... ok
test observer::watchers::meta_watch::tests::test_system_health_evaluation ... ok
test observer::watchers::tests::test_watcher_manager_creation ... ok
...
test result: ok. 37 passed; 0 failed
```

## 架构变化

```
Before (TypeScript Watchers):
┌─────────────────────────────────────────────┐
│ packages/ccode/src/observer/watchers/       │
│   ├── base-watcher.ts (~400行)              │
│   ├── code-watch.ts (~520行)                │
│   ├── world-watch.ts (~430行)               │
│   ├── self-watch.ts (~725行)                │
│   └── meta-watch.ts (~675行)                │
│   Total: ~2750行 TypeScript                 │
└─────────────────────────────────────────────┘

After (Rust Watchers):
┌─────────────────────────────────────────────┐
│ services/zero-cli/src/observer/watchers/    │
│   ├── mod.rs (Watcher trait, Manager)       │
│   ├── base.rs (BaseWatcherState)            │
│   ├── code_watch.rs (git2 native)           │
│   ├── world_watch.rs (reqwest HTTP)         │
│   ├── self_watch.rs (broadcast channels)    │
│   └── meta_watch.rs (introspection)         │
│   Total: ~2770行 Rust                       │
└─────────────────────────────────────────────┘
```

## 文件清单

### 新增文件

| 文件 | 行数 | 描述 |
|------|------|------|
| `watchers/mod.rs` | 300 | Watcher trait, WatcherManager, WatcherStatus |
| `watchers/base.rs` | 200 | BaseWatcherState, WatcherHealth |
| `watchers/code_watch.rs` | 720 | Git 监控, 类型检查 |
| `watchers/world_watch.rs` | 400 | 外部数据监控 |
| `watchers/self_watch.rs` | 600 | Agent 行为监控 |
| `watchers/meta_watch.rs` | 550 | 元观察, 系统健康 |

### 修改文件

| 文件 | 变更 |
|------|------|
| `Cargo.toml` | 添加 git2=0.19, notify=6.1 |
| `observer/mod.rs` | 导出 watchers 模块 |
| `unified_api/observer.rs` | 添加 watcher API 端点 |
| `unified_api/state.rs` | 添加 watcher_manager 字段 |
| `unified_api/mod.rs` | 添加 watcher 路由 |

## 下一步

- [ ] WatcherManager 完整启动/停止逻辑
- [ ] Watcher → ConsensusEngine 事件流连接
- [ ] 端到端集成测试
- [ ] TypeScript watcher 文件清理 (标记 @deprecated)
