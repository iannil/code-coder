# Phase 18: Agent 因果链集成

**状态**: ✅ 已完成
**日期**: 2026-02-24
**作者**: Claude Opus 4.5

---

## 概述

Phase 18 实现了 Agent 系统与因果图数据库（CausalGraph）的深度集成，使 @decision Agent 及其他 Agent 的决策和工具执行能够被自动记录到因果链中，形成可追溯、可分析的决策历史。

## 架构设计

```
@decision Agent                Tool Execution
    │                              │
    ├── recordAgentDecision() ───→ CausalGraph.recordDecision()
    │                              │
    │                              ├── PostToolUse Hook
    │                              │       └── 关联 decisionId
    │                              │
    │                              ├── CausalRecorder.recordToolAction()
    │                              │       └── CausalGraph.recordAction()
    │                              │
    ├── getSuggestions() ←──────── CausalAnalysis.suggestFromHistory()
    │                              │
    └── getHistory() ←───────────── CausalGraph.query()
```

## 实现文件

| 文件 | 操作 | 功能 |
|------|------|------|
| `src/agent/hooks/causal-recorder.ts` | 新建 | 核心因果记录模块 |
| `src/agent/hooks/index.ts` | 新建 | 模块导出 |
| `src/agent/prompt/decision.txt` | 修改 | 增加因果链集成说明 |
| `src/hook/hook.ts` | 修改 | PostToolUse 集成 CausalRecorder |
| `test/unit/agent/causal-recorder.test.ts` | 新建 | 单元测试 (19 测试用例) |

## 核心功能

### 1. CausalRecorder 命名空间

```typescript
export namespace CausalRecorder {
  // 记录 Agent 决策
  recordAgentDecision(ctx): Promise<string>

  // 记录工具执行（PostToolUse Hook 调用）
  recordToolAction(ctx): Promise<string | null>

  // 记录执行结果
  recordOutcome(ctx): Promise<void>

  // 获取智能建议
  getSuggestions(ctx): Promise<Array<Suggestion>>

  // 获取决策历史（格式化显示）
  getHistory(ctx): Promise<string>

  // 会话管理
  getActiveDecisionId(sessionId): string | undefined
  clearSession(sessionId): void
  clearAll(): void
}
```

### 2. 工具到 ActionType 映射

| 工具名 | ActionType |
|--------|------------|
| write, edit, read | file_operation |
| grep, glob, websearch | search |
| bash | tool_execution |
| webfetch | api_call |
| mcp__* | tool_execution |
| 其他 | other |

### 3. PostToolUse Hook 集成

在 `Hook.run()` 函数中，当 lifecycle 为 `PostToolUse` 时，自动调用 `CausalRecorder.recordToolAction()` 记录工具执行。该调用是**非阻塞**的，使用 `.catch()` 处理错误，不影响主流程性能。

```typescript
// src/hook/hook.ts
if (lifecycle === "PostToolUse" && ctx.tool && ctx.sessionID) {
  CausalRecorder.recordToolAction({
    sessionId: ctx.sessionID,
    toolName: ctx.tool,
    toolInput: ctx.input ?? {},
    toolOutput: ctx.output,
  }).catch((err) => {
    log.debug("Failed to record tool action to CausalGraph", { error: err })
  })
}
```

### 4. Decision Prompt 增强

在 `decision.txt` 中添加了因果链集成说明：
- 决策自动记录到因果图
- 可查询历史决策和建议
- 支持决策追溯分析

## 测试覆盖

### 测试用例 (19 个)

1. **recordAgentDecision** (3 个)
   - 记录决策并返回 ID
   - 跟踪活动决策
   - 包含上下文信息

2. **recordToolAction** (5 个)
   - 无活动决策时返回 null
   - 有决策时记录动作
   - 工具名到 ActionType 映射
   - 包含输出信息
   - MCP 工具处理

3. **recordOutcome** (3 个)
   - 记录成功结果
   - 记录失败结果（含错误）
   - 包含度量指标

4. **getSuggestions** (2 个)
   - 返回格式化建议
   - 无建议时返回空数组

5. **getHistory** (2 个)
   - 格式化历史显示
   - 无历史时返回提示

6. **会话管理** (2 个)
   - 清除单个会话
   - 清除所有跟踪数据

7. **输入摘要** (2 个)
   - 文件路径摘要
   - 长命令截断

### 测试结果

```
✓ 19 pass
✓ 0 fail
✓ 36 expect() calls
✓ 103ms 执行时间
```

## 使用方式

### 对于 @decision Agent

```typescript
// 记录决策
const decisionId = await CausalRecorder.recordAgentDecision({
  sessionId: "session-123",
  agentId: "decision",
  prompt: "用户问题",
  reasoning: "CLOSE 分析...",
  confidence: 0.85,
})

// 获取建议
const suggestions = await CausalRecorder.getSuggestions({
  agentId: "decision",
  prompt: "当前问题",
})

// 查看历史
const history = await CausalRecorder.getHistory({
  agentId: "decision",
  limit: 10,
})
```

### 自动记录（无需显式调用）

工具执行会在 PostToolUse Hook 中自动记录，前提是该会话已有活动决策。

## 架构覆盖率

Phase 18 不改变架构覆盖率（已达 99.9%），但增强了现有组件的协同能力：
- 因果图与 Agent 系统深度集成
- 决策追溯可视化
- 智能建议基于真实历史数据

## 后续工作

1. **API 端点**: 可考虑添加 `/api/causal/agent-history` 端点供前端查询
2. **可视化**: 在 TUI 或 Web 界面展示决策因果链图
3. **反馈闭环**: 支持用户对决策结果进行评价，丰富 Outcome 数据

---

## 修改时间线

| 时间 | 操作 |
|------|------|
| 2026-02-24 17:20 | 开始 Phase 18 实现 |
| 2026-02-24 17:25 | 创建 causal-recorder.ts |
| 2026-02-24 17:26 | 创建 index.ts 导出模块 |
| 2026-02-24 17:27 | 更新 decision.txt prompt |
| 2026-02-24 17:28 | 集成 Hook.run() |
| 2026-02-24 17:30 | 创建并运行单元测试 (19 pass) |
| 2026-02-24 17:32 | 完成文档编写 |
