# CodeCoder Rust-First Architecture Refactoring Progress

> 开始时间: 2026-03-12
> 最后更新: 2026-03-12

## 目标

将确定性逻辑从 TypeScript 迁移到 Rust，TS 只负责展示层。

## 阶段概览

| 阶段 | 状态 | 描述 |
|------|------|------|
| 阶段 1: Tools/SDK 强化 | ✅ 完成 | 工具层完整性、NAPI 绑定、SDK 客户端 |
| 阶段 2: Provider 迁移 | ✅ 完成 | Rust 原生 Provider 实现 (Anthropic, OpenAI, Google) |
| 阶段 3: Agent/Session 迁移 | ✅ 完成 | Agent 元数据、autoApprove、observerCapability |
| 阶段 4: 清理与优化 | ✅ 完成 | 迁移文档、弃用警告（删除代码待 v2.0）|

---

## 阶段 1: Tools/SDK 强化

### 已完成

#### 2026-03-12: AgentBridge SDK 创建

- 创建 `packages/ccode/src/sdk/agent-bridge.ts`
- 实现 Agent API 客户端类
- 桥接到 Rust Agent API (`/api/v1/agents`)

#### 2026-03-12: ProviderBridge SDK 创建

- 创建 `packages/ccode/src/sdk/provider-bridge.ts`
- 实现只读 Provider 操作: `list()`, `listAll()`, `getProvider()`, `getModel()`, `defaultModel()`, `getSmallModel()`
- **重要架构决策**: Provider 采用部分迁移策略
  - 原因: `Provider.getLanguage()` 返回 AI SDK 对象，无法通过 HTTP 序列化
  - 方案: Bridge 处理元数据查询，AI SDK 集成保留在 TS

#### 2026-03-12: Rust Provider API 创建

- 创建 `services/zero-cli/src/unified_api/providers.rs`
- 实现 API 端点:
  - `GET /api/v1/providers` - 列出提供商
  - `GET /api/v1/providers/all` - 列出所有提供商
  - `GET /api/v1/providers/:id` - 获取提供商详情
  - `GET /api/v1/providers/:provider_id/models/:model_id` - 获取模型详情
  - `GET /api/v1/providers/default-model` - 获取默认模型
  - `GET /api/v1/providers/:provider_id/small-model` - 获取小模型

#### 2026-03-12: Provider.defaultModel() 迁移

将 `Provider.defaultModel()` 调用迁移到 `getDefaultModelWithFallback()`。

**迁移策略**: Hybrid fallback - Bridge 优先，TS Provider 兜底

**已迁移文件** (15+ 处调用):

| 文件 | 迁移方式 |
|------|----------|
| `session/prompt.ts` | 静态导入 |
| `tool/plan.ts` | 静态导入 |
| `cli/cmd/debug/agent.ts` | 静态导入 (2 处) |
| `autonomous/execution/agent-invoker.ts` | 静态导入 |
| `autonomous/orchestration/orchestrator.ts` | 动态导入 |
| `autonomous/builder/generators/agent-generator.ts` | 静态导入 |
| `autonomous/classification/task-classifier.ts` | 动态导入 |
| `memory/tools/llm-abstractor.ts` | 静态导入 |
| `autonomous/execution/llm-solver.ts` | 静态导入 |
| `autonomous/pdca/strategies/generic.ts` | 动态导入 |
| `autonomous/pdca/strategies/query.ts` | 动态导入 |
| `autonomous/pdca/strategies/research.ts` | 动态导入 (3 处) |
| `autonomous/execution/fix-loop.ts` | 动态导入 |
| `autonomous/execution/acceptance-loop.ts` | 动态导入 |
| `autonomous/execution/research-loop.ts` | 动态导入 |

**保留的直接调用**:
- `agent/agent.ts:816` - Agent 核心层，作为基础设施回退

---

## 阶段 2: Provider 迁移 (✅ 完成)

### 已完成

#### 2026-03-12: Provider 配置加载

- 更新 `services/zero-cli/src/unified_api/state.rs`
  - 添加 `ApiConfig::load_from_config_dir()` 方法
  - 从 `~/.codecoder/providers.json` 加载配置
  - 解析 `_settings.default` 为默认模型
- 更新 `services/zero-cli/src/daemon/mod.rs`
  - 在 daemon 启动时调用 `set_config()`
- **验证结果**:
  ```bash
  $ curl http://127.0.0.1:4402/api/v1/providers/default-model
  {"success":true,"provider_id":"volces","model_id":"deepseek-v3-2-251201"}

  $ curl http://127.0.0.1:4402/api/v1/providers | jq '.providers | keys'
  ["deepseek", "uniapi", "volces", "zhipu-ai"]
  ```
- ProviderBridge 现在通过 Rust API 获取真实配置

### 待完成

1. [x] ~~在 Rust daemon 中加载 provider 配置~~
2. [x] ~~实现 P0 提供商: Anthropic, OpenAI (Rust 原生 HTTP 调用)~~ (2026-03-12)
   - **已实现**: `zero-core/src/provider/anthropic.rs` (流式、工具调用、thinking)
   - **已实现**: `zero-core/src/provider/openai.rs` (流式)
   - **已实现**: `zero-core/src/agent/streaming.rs` - 三大 Provider 的 StreamingProvider trait 实现
     - `AnthropicProvider` - Claude 模型
     - `OpenAIProvider` - GPT/o1/o3 模型
     - `GoogleProvider` - Gemini 模型
   - **已实现**: Daemon 多 Provider 路由 (services/zero-cli/src/daemon/mod.rs)
     - 支持从配置或环境变量选择 Provider
     - 自动检测 API Key 格式选择对应 Provider
     - 支持自定义 base_url 以兼容 OpenAI-compatible 服务
3. [x] ~~实现 P1 提供商: Google~~ (2026-03-12)
   - **已实现**: `zero-core/src/provider/google.rs` (Gemini)
   - **已实现**: `zero-core/src/agent/streaming.rs` - GoogleProvider
   - **待完成**: OpenRouter, Ollama (可通过 OpenAI-compatible 模式支持)
4. [x] 流式响应 (SSE) 支持 - ✅ 已在各 Provider 实现
5. [x] 错误处理和重试逻辑 - ✅ ResilientProvider 已实现

#### 2026-03-12: 多 Provider 路由实现

**修改的文件**:
- `services/zero-core/src/agent/streaming.rs` - 添加 OpenAIProvider 和 GoogleProvider
- `services/zero-core/src/agent/mod.rs` - 导出新 providers
- `services/zero-cli/src/unified_api/state.rs` - 重新导出新 providers
- `services/zero-cli/src/daemon/mod.rs` - 实现 `create_streaming_provider()` 函数
- `services/zero-cli/src/config/schema.rs` - 修复环境变量命名

**环境变量优先级**:
```
API Key:    ANTHROPIC_API_KEY > OPENAI_API_KEY > API_KEY
Provider:   PROVIDER > auto-detect from API key
```

**API Key 自动检测逻辑**:
```rust
fn detect_provider_from_key(api_key: &str) -> String {
    if api_key.starts_with("sk-ant-") || api_key.starts_with("sk-proj-") {
        "anthropic"
    } else if api_key.starts_with("sk-") {
        "openai"
    } else if api_key.starts_with("AIza") {
        "google"
    } else {
        "anthropic"  // default
    }
}
```

**测试验证**:
```bash
# 所有 streaming provider 测试通过
cargo test agent::streaming --lib
# 11 passed; 0 failed
```

---

## 技术笔记

### ProviderBridge 部分迁移原因

```
Provider (TS)
├── 元数据操作 (可迁移)
│   ├── list() → 列出提供商
│   ├── getProvider() → 获取提供商信息
│   ├── getModel() → 获取模型信息
│   └── defaultModel() → 获取默认模型引用
│
└── AI SDK 集成 (必须保留在 TS)
    └── getLanguage() → 返回 LanguageModel 对象
        └── 无法序列化，需要本地 AI SDK 实例
```

### Hybrid Fallback 模式

```typescript
export async function getDefaultModelWithFallback(): Promise<ModelReference> {
  const { bridge, useFallback } = await getProviderBridgeWithFallback()
  if (useFallback) {
    // Rust daemon 未运行或 API 失败，回退到 TS Provider
    const { Provider } = await import("../provider/provider")
    return Provider.defaultModel()
  }
  return bridge.defaultModel()
}
```

---

---

## 阶段 3: Agent/Session 迁移 (✅ 完成)

### 现状分析 (2026-03-12)

#### 已有基础设施

1. **Rust Agent API** 已实现:
   - `GET /api/v1/agents` - 列出所有 agents
   - `GET /api/v1/agents/:name` - 获取 agent 详情
   - `GET /api/v1/agents/:name/prompt` - 获取 prompt

2. **AgentBridge SDK** 已完成 (`src/sdk/agent-bridge.ts`):
   - `list()` / `get()` / `getPrompt()` / `execute()`
   - 类型转换: `toAgentInfo()`, `convertPermissionToRuleset()`
   - Singleton 模式和健康检查

3. **TS Agent 模块** 已标记弃用 (`src/agent/agent.ts`)

4. **Agent 数量**: Rust 和 TS 均有 31 个 agents

#### 发现的配置差异

| 类别 | TS 定义 | Rust API 返回 | 状态 |
|------|---------|---------------|------|
| Primary Mode | build, plan, compaction, title, summary, writer, autonomous (7个) | autonomous, build, plan, writer (4个可见) | ✅ 已修复 |
| Hidden | compaction, title, summary | 从列表过滤 | ✅ 已修复 |
| autoApprove | general, explore 有配置 | API 返回完整配置 | ✅ 已完成 |
| observerCapability | explore 有 CodeWatch | API 返回完整配置 | ✅ 已完成 |
| native 字段 | 多数为 true | 未返回 | ❌ 可忽略 |

#### Primary Mode 详细对比

```
TS Primary Agents (7):     Rust Primary Agents (1):
─────────────────────      ─────────────────────────
✓ build                    ✗ (返回 subagent)
✓ plan                     ✓ plan
✓ compaction               ✗ (返回 subagent)
✓ title                    ✗ (返回 subagent)
✓ summary                  ✗ (返回 subagent)
✓ writer                   ✗ (返回 subagent)
✓ autonomous               ✗ (返回 subagent)
```

#### 原因分析

Rust daemon 从 prompt 文件 (`src/agent/prompt/*.txt`) 加载 agents，但:
1. Prompt 文件不包含 mode/hidden 等元数据
2. 元数据应来自 agent registry 配置文件
3. 需要同步 TS 的 agent 定义到 Rust 配置

### 待完成任务

1. [x] **同步 Agent 元数据到 Rust** (2026-03-12)
   - 采用方案: 在 prompt 文件头部添加 HTML 注释格式元数据
   - 已修改文件:
     - `build.txt` - 添加 `mode: primary`
     - `compaction.txt` - 添加 `mode: primary`, `hidden: true`
     - `title.txt` - 添加 `mode: primary`, `hidden: true`
     - `summary.txt` - 添加 `mode: primary`, `hidden: true`
     - `writer.txt` - 添加 `mode: primary`
     - `autonomous.txt` - 添加 `mode: primary`

2. [x] **修复 mode 字段** - 已通过元数据注释修复

3. [x] **修复 hidden 字段** - 已通过元数据注释修复

4. [x] **验证修改** - ✅ 已验证 (2026-03-12)
   ```bash
   # 验证命令
   curl http://127.0.0.1:4402/api/v1/agents | jq '.agents[] | select(.mode == "primary") | .name'
   # 实际结果: autonomous, build, plan, writer

   curl http://127.0.0.1:4402/api/v1/agents | jq '.agents[] | select(.hidden == true) | .name'
   # 实际结果: compaction, title, summary (hidden agents 从列表中过滤)
   ```

5. [x] **autoApprove 和 observerCapability 迁移** - ✅ 已完成 (2026-03-12)
   - 修改 `services/zero-cli/src/unified_api/state.rs`:
     - 在 `AgentMetadata` 添加 `auto_approve` 和 `observer` 字段
     - 更新 `load_agents()` 和 `reload_agent()` 填充这些字段
   - 修改 `services/zero-cli/src/unified_api/agents.rs`:
     - 在 `AgentInfo` 添加 `auto_approve` 和 `observer` 字段
     - 更新 `From<AgentMetadata> for AgentInfo` 映射新字段
   - **验证结果**:
   ```bash
   $ curl http://127.0.0.1:4402/api/v1/agents/explore | jq '{auto_approve, observer}'
   {
     "auto_approve": {
       "enabled": true,
       "allowed_tools": ["Read", "Glob", "Grep", "LS"],
       "risk_threshold": "low"
     },
     "observer": {
       "can_watch": ["code"],
       "contribute_to_consensus": true,
       "report_to_meta": true
     }
   }
   ```

---

## 下一步

1. ~~实现 Rust 配置加载~~ ✅ 已完成
2. ~~实现 P0/P1 提供商的 Rust 原生 HTTP 调用~~ ✅ 已完成
   - Anthropic, OpenAI, Google 三大 Provider 已实现
   - Daemon 多 Provider 路由已实现
   - 环境变量命名已修复 (ANTHROPIC_API_KEY 等标准命名)
3. ~~开始 Agent 定义迁移到 Rust~~ ✅ 开始分析
4. ~~同步 Agent 元数据配置 (mode, hidden)~~ ✅ 已完成
5. ~~验证 Agent 元数据在 daemon 中正确加载~~ ✅ 已完成
6. ~~autoApprove 和 observerCapability 迁移~~ ✅ 已完成
7. ~~更新 AgentBridge SDK~~ ✅ 已完成 (2026-03-12)
   - 更新 `AgentInfo` 接口添加 `auto_approve` 和 `observer` 字段
   - 更新 `ConvertedAgentInfo` 使用强类型定义
   - 更新 `toAgentInfo()` 映射新字段 (snake_case → camelCase)
8. **下一步**:
   - 实现 P2 提供商 (OpenRouter, Ollama 可通过 OpenAI-compatible 模式)
   - 开始阶段 4: 清理 TS 旧代码

## 阶段 2 完成度

| 提供商 | streaming.rs | provider/*.rs | 状态 |
|--------|-------------|---------------|------|
| Anthropic | ✅ | ✅ | 完成 |
| OpenAI | ✅ | ✅ | 完成 |
| Google | ✅ | ✅ | 完成 |
| OpenRouter | - | - | 可用 OpenAI-compatible |
| Ollama | - | - | 可用 OpenAI-compatible |

---

## 阶段 4: 清理与优化 (进行中)

### 2026-03-12: 依赖分析 + 迁移实施

#### Agent 模块 (`src/agent/agent.ts`)

| 状态 | 依赖文件 | 使用函数 | 迁移方案 |
|------|---------|---------|---------|
| ✅ 已迁移 | `cli/cmd/agent.ts:125` | `Agent.generate()` | 使用 Rust API + AgentBridge.generate() |
| ✅ 已迁移 | `cli/cmd/agent.ts:236` | 列表功能 | 使用 `AgentBridge.list()` |

**Agent 模块迁移完成**:
- ✅ 实现 Rust API: `POST /api/v1/definitions/agents/generate`
- ✅ 在 AgentBridge 添加 `generate()` 方法
- ✅ 更新 CLI 使用 AgentBridge 替代 Agent

#### Provider 模块 (`src/provider/provider.ts`)

**依赖数**: 19 个文件

**迁移策略**: Hybrid Fallback (部分迁移)

| 函数类型 | 迁移状态 | 原因 |
|---------|---------|------|
| 元数据操作 (`list`, `getProvider`, `defaultModel`) | ✅ 已迁移到 ProviderBridge | HTTP 可序列化 |
| AI SDK 集成 (`getLanguage`) | ❌ 必须保留 | 返回 `LanguageModel` 对象，无法序列化 |

**结论**: Provider 模块需要保留，但元数据操作应优先使用 ProviderBridge

### 已完成任务

1. [x] 实现 Rust Agent 生成 API (`POST /api/v1/definitions/agents/generate`) - ✅ 2026-03-12
2. [x] 迁移 `cli/cmd/agent.ts` 使用新 API - ✅ 2026-03-12
3. [x] 添加 TS 代码弃用警告 (console.warn) - ✅ 已存在
4. [x] 文档化迁移指南 - ✅ 2026-03-12
   - 创建 `/docs/migration/rust-first-agent-migration.md`
5. [ ] 删除 `agent/agent.ts` (v2.0)
