# CodeCoder 架构重构进展报告

> 日期: 2026-03-10
> 状态: **Phase 8 完成**
> 延续自: Phase 1 (API Server 合并)

---

## 完成情况概览

| Phase | 描述 | 状态 | 完成时间 |
|-------|------|------|----------|
| Phase 1 | API Server 合并 | ✅ 完成 | 之前完成 |
| Phase 2 | Agent Dispatcher 迁移 | ✅ 完成 | 2026-03-10 |
| Phase 3 | Memory System 完善 | ✅ 完成 | 2026-03-10 |
| Phase 4 | Session 管理扩展 | ✅ 完成 | 2026-03-10 |
| Phase 5 | 集成测试 | ✅ 完成 | 2026-03-10 |
| Phase 6 | 工具执行循环 | ✅ 完成 | 2026-03-10 |
| Phase 7 | Agent 定义迁移 | ✅ 完成 | 2026-03-10 |
| Phase 8 | Gear System 迁移 | ✅ 完成 | 2026-03-10 |

---

## Phase 2: Agent Dispatcher 实现

### 修改文件

- `services/zero-cli/src/unified_api/state.rs`
- `services/zero-cli/src/unified_api/agents.rs`
- `services/zero-cli/src/daemon/mod.rs`

### 关键变更

1. **Provider 集成**
   - 在 `UnifiedApiState` 中添加 `llm_provider` 字段
   - 新增 `with_provider()` 构造方法
   - 导出 streaming 类型: `AnthropicProvider`, `StreamingProvider`, `StreamEvent`, `Message`, `Role` 等

2. **dispatch_agent 实现**
   - 从 session 获取历史消息并转换为 `Message` 类型
   - 从 `ToolRegistry` 获取工具并转换为 `ToolDef`
   - 支持同步和流式(SSE)两种响应模式
   - 自动保存对话消息到 session

3. **Daemon 初始化**
   - 根据 `config.api_key` 创建 `AnthropicProvider`
   - 使用 `UnifiedApiState::with_provider()` 初始化状态

### 新增响应类型

```rust
pub struct DispatchAgentSyncResponse {
    pub success: bool,
    pub request_id: String,
    pub response: String,
    pub usage: Option<TokenUsage>,
    pub error: Option<String>,
}
```

---

## Phase 3: Memory System 完善

### 修改文件

- `services/zero-cli/src/unified_api/memory.rs`

### 关键变更

1. **consolidate() 实现**
   - 读取过去 7 天的每日笔记
   - 提取带 tag 的条目
   - 按 tag 分类合并到长期记忆
   - 自动去重避免重复条目

### 整合逻辑

```
每日笔记 (带 #tag)
      │
      ▼
按 tag 分组
      │
      ▼
合并到长期记忆对应分类
```

---

## Phase 4: Session 管理扩展

### 修改文件

- `services/zero-cli/src/session/store.rs`
- `services/zero-cli/src/session/mod.rs`
- `services/zero-cli/src/unified_api/sessions.rs`

### 关键变更

1. **新增元数据表**

```sql
CREATE TABLE session_metadata (
    session_key     TEXT PRIMARY KEY,
    title           TEXT,
    project_id      TEXT,
    agent           TEXT,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
```

2. **新增 SessionStore 方法**
   - `get_metadata(session_key)` - 获取会话元数据
   - `set_metadata(session_key, title, project_id, agent)` - 设置元数据
   - `set_title()`, `set_project_id()`, `set_agent()` - 便捷方法
   - `list_sessions_with_metadata()` - 列出会话及元数据
   - `delete_metadata()` - 删除元数据

3. **API 端点更新**
   - `GET /api/v1/sessions` - 返回元数据字段，支持 `project_id` 过滤
   - `POST /api/v1/sessions` - 保存元数据
   - `GET /api/v1/sessions/:id` - 返回元数据
   - `PATCH /api/v1/sessions/:id` - 更新元数据
   - `DELETE /api/v1/sessions/:id` - 删除元数据

### 新增类型

```rust
pub struct SessionMetadata {
    pub session_key: String,
    pub title: Option<String>,
    pub project_id: Option<String>,
    pub agent: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
```

---

## Phase 5: 集成测试

### 测试结果

```
running 580 tests
test result: ok. 580 passed; 0 failed; 5 ignored

running 12 tests (dockerignore)
test result: ok. 12 passed; 0 failed; 0 ignored

running 7 tests (memory_comparison)
test result: ok. 7 passed; 0 failed; 0 ignored

running 10 tests (sandbox_integration)
test result: ok. 0 passed; 0 failed; 10 ignored (requires Docker)

running 5 doc-tests
test result: ok. 1 passed; 0 failed; 4 ignored
```

### 构建验证

```
cargo build -p zero-cli: SUCCESS
cargo check: SUCCESS (18 warnings - unused imports)
```

---

## 架构验收

### 新架构验证

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Rust Core (4402)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   /api/v1/sessions/*   ───► SessionStore + SessionMetadata                  │
│   /api/v1/agents/*     ───► AgentExecutor + StreamingProvider               │
│   /api/v1/memory/*     ───► Memory System (daily + long-term)               │
│   /api/v1/tasks/*      ───► Task Management                                 │
│   /api/v1/config/*     ───► Config Management                               │
│   /api/v1/prompts/*    ───► Prompt Hot-loading                              │
│                                                                              │
│   /gateway/*           ───► zero-hub (Auth, RBAC)                           │
│   /channels/*          ───► zero-hub (IM integrations)                      │
│   /workflow/*          ───► zero-hub (Scheduling, Webhooks)                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### API 端点验证清单

- [x] `GET /api/v1/sessions` - 列出会话（带元数据）
- [x] `POST /api/v1/sessions` - 创建会话（保存元数据）
- [x] `GET /api/v1/sessions/:id` - 获取会话详情
- [x] `PATCH /api/v1/sessions/:id` - 更新会话元数据
- [x] `DELETE /api/v1/sessions/:id` - 删除会话
- [x] `GET /api/v1/sessions/:id/messages` - 获取消息
- [x] `POST /api/v1/sessions/:id/messages` - 发送消息
- [x] `POST /api/v1/sessions/:id/fork` - 分叉会话
- [x] `POST /api/v1/sessions/:id/compact` - 压缩会话
- [x] `GET /api/v1/agents` - 列出 Agent
- [x] `GET /api/v1/agents/:name` - 获取 Agent 详情
- [x] `POST /api/v1/agents/dispatch` - 执行 Agent（同步/流式）
- [x] `GET /api/v1/memory/daily` - 列出每日笔记
- [x] `GET /api/v1/memory/daily/:date` - 获取指定日期笔记
- [x] `POST /api/v1/memory/daily` - 追加每日笔记
- [x] `GET /api/v1/memory/long-term` - 获取长期记忆
- [x] `PUT /api/v1/memory/category/:name` - 更新分类
- [x] `POST /api/v1/memory/category/:name/merge` - 合并到分类
- [x] `POST /api/v1/memory/consolidate` - 整合每日笔记到长期记忆

---

## 下一步计划

1. ~~**工具执行循环**: 当前 dispatch 只做单轮 LLM 调用，需要实现完整的工具执行循环（调用工具后继续对话）~~ ✅ 已完成
2. **清理警告**: 修复 unused imports 警告
3. **文档更新**: 更新 API 文档和使用说明
4. **性能优化**: 对高频 API 进行性能测试和优化

---

## Phase 6: 工具执行循环实现 (2026-03-10 14:30)

### 修改文件

- `services/zero-cli/src/unified_api/agents.rs`

### 核心实现

1. **run_agent_loop()** - 多轮工具执行循环
   ```rust
   async fn run_agent_loop(
       state: Arc<UnifiedApiState>,
       provider: Arc<dyn StreamingProvider>,
       mut request: StreamRequest,
       max_iterations: usize,
       tool_timeout_secs: u64,
   ) -> anyhow::Result<AgentLoopResult>
   ```

   执行流程:
   ```
   User → LLM → ToolCall? ─Yes→ 执行工具 → 添加结果 → LLM → (循环)
                   │
                  No
                   │
                   └─→ 返回最终文本响应
   ```

2. **execute_tool()** - 工具执行函数
   - 从 ToolRegistry 查找工具
   - 支持超时控制 (tokio::time::timeout)
   - 错误友好返回给 LLM 自行处理

3. **流式工具循环** - SSE 模式也支持完整工具执行
   - 实时流式输出 LLM 响应
   - 执行工具后发送 ToolResult 事件
   - 继续下一轮 LLM 调用
   - 累积 token 使用量

### API 变更

`DispatchAgentRequest` 新增字段:
```rust
pub struct DispatchAgentRequest {
    // ... 已有字段

    /// Max tool execution iterations (default: 10)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Tool execution timeout in seconds (default: 30)
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout: u64,
}
```

### 新增类型

```rust
/// Information about a tool call from the LLM
struct ToolCallInfo {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

/// Result of the agent execution loop
struct AgentLoopResult {
    text: String,
    usage: TokenUsage,
    iterations: usize,
}
```

### TokenUsage 增强

```rust
impl TokenUsage {
    /// Merge another usage into this one (accumulate tokens)
    pub fn merge(&mut self, other: &TokenUsage)
}

impl Default for TokenUsage { ... }
```

### 验证结果

```
cargo check -p zero-cli: SUCCESS (0 errors in agents.rs)
cargo test -p zero-cli: 1170+ tests passed
```

### 安全限制

| 配置 | 默认值 | 说明 |
|------|--------|------|
| `max_iterations` | 10 | 防止无限循环 |
| `tool_timeout` | 30s | 单个工具执行超时 |

---

## Phase 7: Agent 定义迁移 (2026-03-10 16:00)

### 目标

将 29 个 Agent 的元数据和配置从 TypeScript (`packages/ccode/src/agent/agent.ts`) 迁移到 Rust，实现:

1. **AgentDefinition** 结构体完整定义 (包含 ObserverCapability, AutoApprove 等)
2. **29 个内置 Agent** 的静态定义
3. **CRUD API 端点** 用于查询和自定义 Agent

### 修改文件

- `services/zero-cli/src/unified_api/definitions.rs` (新建)
- `services/zero-cli/src/unified_api/mod.rs` (添加模块和路由)
- `services/zero-cli/src/unified_api/state.rs` (添加 custom_agents 字段)

### 核心类型

```rust
/// Watcher types in the Observer Network
pub enum WatcherType {
    Code,   // CodeWatch - 代码库扫描
    World,  // WorldWatch - 外部世界
    Self_,  // SelfWatch - Agent 行为
    Meta,   // MetaWatch - 系统健康
}

/// Observer capability for Observer Network participation
pub struct ObserverCapability {
    pub can_watch: Vec<WatcherType>,
    pub contribute_to_consensus: bool,
    pub report_to_meta: bool,
}

/// Auto-approve configuration for safe tool execution
pub struct AutoApproveConfig {
    pub enabled: bool,
    pub allowed_tools: Vec<String>,
    pub risk_threshold: String, // "safe", "low", "medium", "high"
}

/// Agent mode
pub enum AgentMode {
    Subagent,  // 子代理
    Primary,   // 主代理
    All,       // 通用
}

/// Complete agent definition
pub struct AgentDefinition {
    pub name: String,
    pub description: Option<String>,
    pub mode: AgentMode,
    pub native: bool,
    pub hidden: bool,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub color: Option<String>,
    pub model: Option<ModelConfig>,
    pub steps: Option<usize>,
    pub options: AgentOptions,
    pub auto_approve: Option<AutoApproveConfig>,
    pub observer_capability: Option<ObserverCapability>,
}
```

### 内置 Agent 分布

| 分类 | Agent | 数量 |
|------|-------|------|
| Primary Mode | build, plan, writer, autonomous | 4 |
| Hidden System | compaction, title, summary | 3 |
| Engineering Quality | general, explore, code-reviewer, security-reviewer, tdd-guide, architect | 6 |
| Content Creation | expander, proofreader, verifier | 3 |
| Reverse Engineering | code-reverse, jar-code-reverse | 2 |
| 祝融说系列 | observer, decision, macro, trader, picker, miniproduct, ai-engineer, value-analyst | 8 |
| Product & Feasibility | prd-generator, feasibility-assess | 2 |
| Other | synton-assistant | 1 |
| **Total** | | **29** |

### 新增 API 端点

```
GET  /api/v1/definitions/agents          # 列出所有 Agent 定义
GET  /api/v1/definitions/agents/:name    # 获取 Agent 详情 + Prompt
PUT  /api/v1/definitions/agents/:name    # 更新 Agent (热加载)
POST /api/v1/definitions/agents          # 创建自定义 Agent
```

### 测试验证

```
running 8 tests
test unified_api::definitions::tests::test_builtin_agents_count ... ok
test unified_api::definitions::tests::test_builtin_agents_unique_names ... ok
test unified_api::definitions::tests::test_primary_agents ... ok (4 primary)
test unified_api::definitions::tests::test_hidden_agents ... ok (3 hidden)
test unified_api::definitions::tests::test_observer_capable_agents ... ok (14+ agents)
test unified_api::definitions::tests::test_auto_approve_agents ... ok (2 agents)
test unified_api::definitions::tests::test_watcher_type_serialization ... ok
test unified_api::definitions::tests::test_agent_mode_serialization ... ok

test result: ok. 8 passed; 0 failed; 0 ignored
```

### 架构验证

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Rust Core (4402)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   /api/v1/definitions/agents/*  ───► AgentDefinition (29 builtin + custom)  │
│   /api/v1/agents/*              ───► AgentExecutor + StreamingProvider      │
│   /api/v1/sessions/*            ───► SessionStore + SessionMetadata         │
│   /api/v1/memory/*              ───► Memory System (daily + long-term)      │
│   /api/v1/tasks/*               ───► Task Management                        │
│   /api/v1/config/*              ───► Config Management                      │
│   /api/v1/prompts/*             ───► Prompt Hot-loading                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 下一步计划

### Observer Network 迁移 (Phase 8)

1. **Gear System** — 五档控制 (P/N/D/S/M) + 三旋钮 (Observe/Decide/Act)
2. **四大 Watcher** — CodeWatch, WorldWatch, SelfWatch, MetaWatch
3. **Consensus Engine** — 共识引擎 + 世界模型
4. **CLOSE 评估** — 五维评估框架

### 待完成文件结构

```
services/zero-cli/src/
├── gear/              # [新增] 档位控制
│   ├── mod.rs
│   ├── dials.rs       # 三旋钮
│   ├── presets.rs     # P/N/D/S/M 预设
│   └── close.rs       # CLOSE 评估
│
├── observer/          # [新增] 观察者网络
│   ├── mod.rs
│   ├── types.rs       # 核心类型
│   ├── watchers/      # 四大 Watcher
│   │   ├── code_watch.rs
│   │   ├── world_watch.rs
│   │   ├── self_watch.rs
│   │   └── meta_watch.rs
│   ├── consensus/     # 共识引擎
│   │   ├── engine.rs
│   │   ├── patterns.rs
│   │   └── world_model.rs
│   └── events.rs      # 事件流
```

---

## 验证命令

```bash
# Phase 7 验证
cargo check -p zero-cli
cargo test -p zero-cli definitions
curl http://localhost:4402/api/v1/definitions/agents | jq '.agents[].name'
```

---

## Phase 8: Gear System 迁移 (2026-03-10 18:30)

### 目标

将档位控制系统从 TypeScript 迁移到 Rust，包括:

1. **Three Dials** — Observe/Decide/Act 三旋钮 (0-100%)
2. **Gear Presets** — P/N/D/S/M 五档预设
3. **CLOSE Evaluator** — 五维评估框架 (Convergence/Leverage/Optionality/Surplus/Evolution)
4. **API Routes** — Gear 控制 HTTP 端点

### 新建文件

| 文件 | 行数 | 描述 |
|------|------|------|
| `services/zero-cli/src/gear/mod.rs` | 280+ | GearState 主模块 |
| `services/zero-cli/src/gear/dials.rs` | 540+ | Dial, ThreeDials 类型 |
| `services/zero-cli/src/gear/presets.rs` | 440+ | GearPreset, GEAR_PRESETS 静态数据 |
| `services/zero-cli/src/gear/close.rs` | 730+ | CLOSEEvaluator, CLOSEEvaluation |
| `services/zero-cli/src/unified_api/gear.rs` | 330+ | API 路由处理器 |

### 修改文件

- `services/zero-cli/src/lib.rs` — 添加 `pub mod gear;`
- `services/zero-cli/src/main.rs` — 添加 `mod gear;`
- `services/zero-cli/src/unified_api/mod.rs` — 添加 gear 路由
- `services/zero-cli/src/unified_api/state.rs` — 添加 `GearState` 字段

### 核心类型

```rust
/// Gear presets for intuitive control
pub enum GearPreset {
    P,  // Park - 系统休眠，无资源消耗
    N,  // Neutral - 纯观察，不干预
    D,  // Drive - 日常平衡模式（默认）
    S,  // Sport - 高自主模式
    M,  // Manual - 三旋钮自定义
}

/// Simple dial values (0-100)
pub struct DialValues {
    pub observe: u8,
    pub decide: u8,
    pub act: u8,
}

/// Gear system state
pub struct GearState {
    pub current_gear: Arc<RwLock<GearPreset>>,
    pub dials: Arc<RwLock<ThreeDials>>,
    pub close_evaluator: Arc<RwLock<CLOSEEvaluator>>,
    pub auto_switch_enabled: Arc<RwLock<bool>>,
}

/// CLOSE five-dimension evaluation
pub struct CLOSEEvaluation {
    pub convergence: CLOSEDimension,
    pub leverage: CLOSEDimension,
    pub optionality: CLOSEDimension,
    pub surplus: CLOSEDimension,
    pub evolution: CLOSEDimension,
    pub total: f32,
    pub risk: f32,
    pub confidence: f32,
    pub recommended_gear: GearPreset,
    pub trend: CLOSETrend,
    pub timestamp: DateTime<Utc>,
}
```

### 预设值表

| 档位 | Observe | Decide | Act | 风险等级 | 适用场景 |
|------|---------|--------|-----|----------|----------|
| P | 0 | 0 | 0 | None | 系统维护，明确暂停 |
| N | 50 | 0 | 0 | Low | 监控但不行动，学习阶段 |
| D | 70 | 60 | 40 | Medium | 日常开发，标准操作 |
| S | 90 | 80 | 70 | High | 时间紧迫，批量处理 |
| M | 自定义 | 自定义 | 自定义 | Medium | 实验性配置 |

### 新增 API 端点

```
GET  /api/v1/gear/current       # 获取当前档位和旋钮值
POST /api/v1/gear/switch        # 切换档位 (需确认切换到 S 档)
POST /api/v1/gear/dials         # 设置三旋钮 (自动切换到 M 档)
POST /api/v1/gear/dial          # 设置单个旋钮
GET  /api/v1/gear/presets       # 获取所有预设详情
GET  /api/v1/gear/presets/:gear # 获取特定预设详情
GET  /api/v1/gear/close         # 获取当前 CLOSE 评估
POST /api/v1/gear/close         # 运行 CLOSE 评估
POST /api/v1/gear/auto-switch   # 启用/禁用自动档位切换
```

### CLOSE 五维评估

| 维度 | 英文 | 说明 | 评估因子 |
|------|------|------|----------|
| C | Convergence | 收敛程度 | 快照置信度、构建状态、会话健康 |
| L | Leverage | 杠杆效应 | 高影响机会数、活跃 Agent 数 |
| O | Optionality | 选择权 | 时间余量、可回退选项、错误预算 |
| S | Surplus | 可用余量 | 共识强度、资源可用性、健康 Watcher |
| E | Evolution | 演化潜力 | 收敛趋势、技能组合、异常比例 |

### 测试验证

```
running 32 tests
test gear::dials::tests::test_dial_adaptive_bounds ... ok
test gear::dials::tests::test_dial_adjust ... ok
test gear::dials::tests::test_dial_is_high ... ok
test gear::dials::tests::test_dial_value_clamping ... ok
test gear::dials::tests::test_three_dials_autonomy_score ... ok
test gear::dials::tests::test_three_dials_detect_gear ... ok
test gear::dials::tests::test_three_dials_from_gear ... ok
test gear::dials::tests::test_three_dials_is_parked ... ok
test gear::dials::tests::test_three_dials_set_dial_switches_to_manual ... ok
test gear::close::tests::test_close_dimension_factors ... ok
test gear::close::tests::test_close_evaluator_default ... ok
test gear::close::tests::test_close_evaluator_healthy_input ... ok
test gear::close::tests::test_close_evaluator_history ... ok
test gear::close::tests::test_close_evaluator_risky_input ... ok
test gear::close::tests::test_close_trend ... ok
test gear::presets::tests::test_gear_allows_autonomous ... ok
test gear::presets::tests::test_gear_preset_count ... ok
test gear::presets::tests::test_gear_preset_display ... ok
test gear::presets::tests::test_gear_preset_from_str ... ok
test gear::presets::tests::test_gear_presets_values ... ok
test gear::presets::tests::test_gear_risk_level ... ok
test gear::presets::tests::test_get_all_gear_presets_order ... ok
test gear::presets::tests::test_suggest_gear ... ok
test gear::presets::tests::test_validate_gear_transition_normal ... ok
test gear::presets::tests::test_validate_gear_transition_to_sport ... ok
test gear::tests::test_gear_state_default ... ok
test gear::tests::test_gear_switch ... ok
test gear::tests::test_gear_status ... ok
test gear::tests::test_manual_dial_setting ... ok
test unified_api::gear::tests::test_gear_state_default ... ok
test unified_api::gear::tests::test_gear_switch_request_parse ... ok
test unified_api::gear::tests::test_set_dials_request_parse ... ok

test result: ok. 32 passed; 0 failed; 0 ignored
```

### 总测试数

```
zero-cli library:  620 passed
zero-cli binary:   620 passed
integration:       12 passed
memory comparison: 7 passed
doc-tests:         1 passed

Total: 1260 tests passed
```

### 架构更新

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Rust Core (4402)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   /api/v1/gear/*              ───► GearState (P/N/D/S/M + ThreeDials)       │
│   /api/v1/definitions/agents/*───► AgentDefinition (29 builtin + custom)    │
│   /api/v1/agents/*            ───► AgentExecutor + StreamingProvider        │
│   /api/v1/sessions/*          ───► SessionStore + SessionMetadata           │
│   /api/v1/memory/*            ───► Memory System (daily + long-term)        │
│   /api/v1/tasks/*             ───► Task Management                          │
│   /api/v1/config/*            ───► Config Management                        │
│   /api/v1/prompts/*           ───► Prompt Hot-loading                       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 下一步计划

### Observer Network 迁移 (Phase 9)

1. **四大 Watcher** — CodeWatch, WorldWatch, SelfWatch, MetaWatch
2. **Consensus Engine** — 共识引擎 + 世界模型
3. **Event Stream** — 事件流 + SSE 推送
4. **Mode Controller** — AUTO/MANUAL/HYBRID 模式控制

### 待完成文件结构

```
services/zero-cli/src/
├── observer/          # [新增] 观察者网络
│   ├── mod.rs
│   ├── types.rs       # 核心类型
│   ├── watchers/      # 四大 Watcher
│   │   ├── code_watch.rs
│   │   ├── world_watch.rs
│   │   ├── self_watch.rs
│   │   └── meta_watch.rs
│   ├── consensus/     # 共识引擎
│   │   ├── engine.rs
│   │   ├── patterns.rs
│   │   └── world_model.rs
│   └── events.rs      # 事件流
```

---

## 验证命令

```bash
# Phase 8 验证
cargo check -p zero-cli
cargo test -p zero-cli gear
curl http://localhost:4402/api/v1/gear/current
curl http://localhost:4402/api/v1/gear/presets | jq '.presets[].gear'
```
