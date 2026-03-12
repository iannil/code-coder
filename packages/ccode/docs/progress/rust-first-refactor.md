# CodeCoder Rust-First Architecture Refactoring Progress

> 开始时间: 2026-03-12
> 最后更新: 2026-03-12

## 目标

将确定性逻辑从 TypeScript 迁移到 Rust，TS 只负责展示层。

## 阶段概览

| 阶段 | 状态 | 描述 |
|------|------|------|
| 阶段 1: Tools/SDK 强化 | 进行中 | 工具层完整性、NAPI 绑定、SDK 客户端 |
| 阶段 2: Provider 迁移 | 进行中 | Rust 原生 Provider 实现 |
| 阶段 3: Agent/Session 迁移 | 待开始 | Agent 引擎和 Session 管理迁移 |
| 阶段 4: 清理与优化 | 待开始 | 删除 TS 旧代码 |

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

## 阶段 2: Provider 迁移 (进行中)

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
2. [ ] 实现 P0 提供商: Anthropic, OpenAI (Rust 原生 HTTP 调用)
3. [ ] 实现 P1 提供商: Google, OpenRouter, Ollama
4. [ ] 流式响应 (SSE) 支持
5. [ ] 错误处理和重试逻辑

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

## 下一步

1. ~~实现 Rust 配置加载~~ ✅ 已完成
2. 实现 P0 提供商的 Rust 原生 HTTP 调用 (Anthropic, OpenAI)
   - 目前只加载了配置，实际调用仍通过 TS AI SDK
   - 需要实现 Rust 端的 streaming HTTP client
3. 开始 Agent 定义迁移到 Rust
