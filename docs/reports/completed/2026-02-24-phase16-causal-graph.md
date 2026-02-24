# Phase 16: 因果链图数据库 (Causal Graph)

**状态**: ✅ 已完成
**日期**: 2026-02-24
**作者**: Claude Code

---

## 概述

Phase 16 实现了因果链图数据库，用于追踪 Agent 决策、行动和结果之间的因果关系。这是 `tech-structure.md` 规范中定义的"图数据库 Neo4j: 因果链/代码依赖"功能的轻量级实现。

## 核心设计

### 三层节点模型

```
┌──────────────┐     causes      ┌──────────────┐    results_in   ┌──────────────┐
│   Decision   │ ─────────────► │    Action    │ ─────────────► │   Outcome    │
│              │                 │              │                 │              │
│ • sessionId  │                 │ • decisionId │                 │ • actionId   │
│ • agentId    │                 │ • actionType │                 │ • status     │
│ • prompt     │                 │ • description│                 │ • description│
│ • reasoning  │                 │ • input      │                 │ • metrics    │
│ • confidence │                 │ • output     │                 │ • feedback   │
│ • context    │                 │ • duration   │                 │              │
└──────────────┘                 └──────────────┘                 └──────────────┘
```

### 数据结构

**决策节点 (DecisionNode)**
- 记录 Agent 做出的决策
- 包含推理过程和信心指数
- 上下文信息（相关文件、使用的工具）

**行动节点 (ActionNode)**
- 记录决策导致的具体行动
- 支持类型：code_change, tool_execution, api_call, file_operation, search, other
- 包含输入/输出参数和执行时长

**结果节点 (OutcomeNode)**
- 记录行动的最终结果
- 状态：success, failure, partial
- 可量化指标（测试通过数、覆盖率变化等）

## 实现文件

| 文件 | 功能 |
|------|------|
| `src/memory/knowledge/causal-types.ts` | Zod schemas 和 TypeScript 类型定义 |
| `src/memory/knowledge/causal-graph.ts` | 因果图核心逻辑（CRUD、查询、统计） |
| `src/memory/knowledge/causal-analysis.ts` | 模式识别和建议生成 |
| `src/api/server/handlers/causal.ts` | API handlers (20 个端点) |
| `src/api/server/router.ts` | 注册 causal 路由 |
| `test/unit/memory/causal-graph.test.ts` | 39 个单元测试 (全部通过) |

## API 端点

### 记录操作
- `POST /api/v1/causal/decisions` - 记录决策
- `GET /api/v1/causal/decisions/:id` - 获取决策
- `POST /api/v1/causal/actions` - 记录行动
- `GET /api/v1/causal/actions/:id` - 获取行动
- `POST /api/v1/causal/outcomes` - 记录结果
- `GET /api/v1/causal/outcomes/:id` - 获取结果

### 因果链查询
- `GET /api/v1/causal/chain/:id` - 获取完整因果链
- `GET /api/v1/causal/chains?sessionId=xxx` - 获取会话因果链
- `POST /api/v1/causal/query` - 复杂查询

### 分析与建议
- `GET /api/v1/causal/patterns` - 获取因果模式
- `GET /api/v1/causal/patterns/success` - 成功模式
- `GET /api/v1/causal/patterns/failure` - 失败模式
- `GET /api/v1/causal/stats` - 统计信息
- `POST /api/v1/causal/suggest` - 基于历史建议
- `GET /api/v1/causal/trends` - 趋势分析
- `GET /api/v1/causal/insights/:agentId` - Agent 洞察
- `GET /api/v1/causal/lessons/:outcomeId` - 经验教训

### 可视化
- `GET /api/v1/causal/graph` - 图数据
- `GET /api/v1/causal/mermaid` - Mermaid 图表

### 健康检查
- `GET /api/v1/causal/health` - 服务健康状态

## 核心功能

### 1. 因果链记录

```typescript
// 记录决策
const decision = await CausalGraph.recordDecision({
  sessionId: "session_123",
  agentId: "@decision",
  prompt: "Should we refactor this code?",
  reasoning: "The code is complex and needs simplification",
  confidence: 0.85,
  context: {
    files: ["src/main.ts"],
    tools: ["Read", "Grep"],
  },
})

// 记录行动
const action = await CausalGraph.recordAction({
  decisionId: decision.id,
  actionType: "code_change",
  description: "Refactored the main function",
  input: { file: "src/main.ts" },
  duration: 5000,
})

// 记录结果
const outcome = await CausalGraph.recordOutcome({
  actionId: action.id,
  status: "success",
  description: "All tests passed",
  metrics: {
    testsPass: 42,
    testsFail: 0,
    coverageChange: 5,
  },
})
```

### 2. 模式识别

```typescript
// 找到成功模式
const successPatterns = await CausalAnalysis.findSuccessPatterns({
  agentId: "@decision",
  minSuccessRate: 0.7,
})

// 找到失败模式
const failurePatterns = await CausalAnalysis.findFailurePatterns({
  maxSuccessRate: 0.3,
})
```

### 3. 智能建议

```typescript
const suggestions = await CausalAnalysis.suggestFromHistory({
  prompt: "Should we add caching?",
  agentId: "@decision",
  context: { files: ["src/api.ts"] },
})
// 返回类似决策的成功/失败经验
```

### 4. 趋势分析

```typescript
const trends = await CausalAnalysis.analyzeTrends({
  agentId: "@decision",
  periodDays: 7,
})
// 返回成功率趋势、信心度趋势、行动类型变化
```

## 与 @decision Agent 集成示例

```typescript
// 在 @decision agent 中自动记录因果链
async function makeDecision(input: DecisionInput): Promise<Decision> {
  const decision = await analyzeWithCLOSE(input)

  // 记录决策
  await CausalGraph.recordDecision({
    sessionId: input.sessionId,
    agentId: "@decision",
    prompt: input.prompt,
    reasoning: decision.reasoning,
    confidence: decision.confidence,
    context: {
      tools: input.tools,
      files: input.relevantFiles,
    }
  })

  return decision
}
```

## 测试覆盖

```
-----------------------------------------|---------|---------|-------------------
File                                     | % Funcs | % Lines | Uncovered Line #s
-----------------------------------------|---------|---------|-------------------
 src/memory/knowledge/causal-analysis.ts |   97.62 |   92.23 | ...
 src/memory/knowledge/causal-graph.ts    |   97.40 |   98.78 | ...
-----------------------------------------|---------|---------|-------------------

 39 pass
 0 fail
 95 expect() calls
```

## 架构覆盖率更新

| 层级 | 之前 | 之后 |
|------|------|------|
| 全局记忆层 | 98% | **100%** |

### 全局记忆层完成情况

- ✅ 向量 SQLite (sqlite-vec)
- ✅ Markdown 记忆 (memory/daily, memory/MEMORY.md)
- ✅ 审计持久化 (compliance API)
- ✅ Call Graph (代码依赖)
- ✅ 动态工具库 (Dynamic Tool Registry)
- ✅ **因果链图数据库 (Causal Graph)** ← Phase 16

## 后续工作

Phase 16 完成后，系统架构覆盖率预计提升至 **100%**（全局记忆层）。

可考虑的后续方向：
1. **IDE 插件** - 补齐触点层最后 5%
2. **自动抽象化** - 完善自主保底层
3. **因果图可视化 UI** - Web 前端集成
4. **Agent 自动学习** - 基于因果分析自动调整策略

---

*文档生成时间: 2026-02-24*
