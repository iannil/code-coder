# Crazy Agent CLOSE 决策框架集成 - 实施进度

**日期**: 2026-02-12
**状态**: ✅ Phase 1 完成 (MVP - 核心功能)

---

## 一、问题诊断

### 根本原因
**"架构完整但集成断层"**：
- `crazy/` 目录下的所有组件（DecisionEngine、Orchestrator、Executor 等）已经完整实现
- `startCrazyMode()` 函数存在于 `crazy-agent.ts`，但从未被主执行路径调用
- 当用户使用 `@crazy` 或 `--agent crazy` 时，仅 prompt 被加载，CLOSE 决策引擎从未被触发

### 执行流程对比

**之前（存在问题）**：
```
用户输入 (@crazy/--agent crazy)
    ↓
Agent.get("crazy") → 加载 PROMPT_CRAZY (仅告知 LLM "应该用 CLOSE")
    ↓
LLM.stream() → 发送 prompt 给模型
    ↓
Tool.execute() → 执行工具
    ↓
❌ DecisionEngine.evaluate() 从未被调用
```

**现在（已集成）**：
```
用户输入 (@crazy/--agent crazy)
    ↓
Agent.get("crazy") → 加载 PROMPT_CRAZY + CLOSE 决策框架
    ↓
LLM.stream() → 发送 prompt 给模型
    ↓
Tool.execute() → 工具执行前触发 PreToolUse
    ↓
✅ CrazyModeHook.evaluateToolCall() → DecisionEngine.evaluate()
    ↓
├─ 分数高 → Tool.execute()
├─ 分数中 → 执行但带警告
└─ 分数低 → 抛出 HookBlockedError 阻止
```

---

## 二、实施完成情况

### Phase 1: 基础集成 (MVP) ✅

| 步骤 | 文件 | 状态 |
|------|------|------|
| 1.1 | `crazy/integration/hook.ts` (新增) | ✅ 完成 |
| 1.2 | `tool/tool.ts` (第89行后) | ✅ 完成 |
| 1.3 | `crazy/decision/engine.ts` | ✅ 已有日志 |
| 1.4 | 类型检查验证 | ✅ 通过 |

### 新增文件

#### `packages/ccode/src/crazy/integration/hook.ts`

**核心功能**：
- `getEngine(sessionId)` - 获取或创建会话的 DecisionEngine
- `evaluateToolCall(ctx)` - 使用 CLOSE 框架评估工具调用
- `recordError(sessionId)` - 记录会话错误
- `cleanup(sessionId)` - 清理会话资源
- `getSessionStats(sessionId)` - 获取会话统计
- `hasActiveSession(sessionId)` - 检查会话是否有活跃引擎
- `clearAllSessions()` - 清除所有会话（测试用）

**CLOSE 决策逻辑**：
1. 从工具调用提取风险等级和决策类型
2. 构建 CLOSE 评分标准
3. 调用 DecisionEngine.evaluate()
4. 记录决策历史
5. 返回是否允许执行

### 修改文件

#### `packages/ccode/src/tool/tool.ts`

**修改点**：第97行后添加 CLOSE 决策评估

```typescript
// Crazy Mode: Run CLOSE decision evaluation
if (ctx.agent === "crazy") {
  const crazyDecision = await CrazyModeHook.evaluateToolCall({
    sessionId: ctx.sessionID,
    toolName: id,
    toolInput: args as Record<string, unknown>,
  })

  if (!crazyDecision.allowed) {
    throw new HookBlockedError({
      hookName: "CrazyMode",
      message: crazyDecision.decision?.reasoning ?? "Blocked by CLOSE decision framework",
      tool: id,
      lifecycle: "PreToolUse",
    })
  }
}
```

#### `packages/ccode/src/crazy/index.ts`

**新增导出**：
```typescript
// Integration
export { CrazyModeHook } from "./integration/hook"
```

---

## 三、工具风险评估

### 默认风险等级

| 工具 | 风险等级 | 说明 |
|--------|----------|------|
| write, edit, read, glob, grep | low | 基础代码操作 |
| bash | medium | 命令执行 |
| 其他 | medium | 未分类工具 |

### 破坏性操作检测

以下命令模式会被识别为高风险（自动提升风险等级）：
- `rm -rf?`
- `delete /*`
- `truncate`
- `drop table`
- `drop database`

---

## 四、CLOSE 决策评分逻辑

### 工具调用的 CLOSE 评分

| 维度 | 安全操作 | 破坏性操作 | 说明 |
|--------|----------|--------------|------|
| C - 收敛性 | 3 | 8 | 破坏性操作大幅减少可能性空间 |
| L - 杠杆率 | 6-8 | 6 | 读操作杠杆率高（低风险高信息） |
| O - 可选性 | 7 | 3 | 破坏性操作减少未来选项 |
| S - 余量 | 7 | 7 | 资源余量保持稳定 |
| E - 演化 | 5 | 5 | 学习价值保持稳定 |

### 决策阈值

根据 `autonomyLevel` 配置：

| 自主等级 | 批准阈值 | 谨慎阈值 |
|----------|-----------|------------|
| lunatic | 5.0 | 3.0 |
| insane | 5.5 | 3.5 |
| crazy (默认) | 6.0 | 4.0 |
| wild | 6.5 | 4.5 |
| bold | 7.0 | 5.0 |
| timid | 8.0 | 6.0 |

---

## 五、日志输出

### CLOSE 决策日志格式

```
[crazy.integration.hook] INFO: CLOSE Decision evaluating
  sessionId: "xxx"
  tool: "write"
  description: "Execute tool: write"
  riskLevel: "low"

[crazy.integration.hook] INFO: CLOSE Decision: PROCEED
  sessionId: "xxx"
  tool: "write"
  score: 7.50
  breakdown: { C: "3.0", L: "6.0", O: "7.0", S: "7.0", E: "5.0" }
  approved: true
  reasoning: "CLOSE Score: 7.50/10"
```

---

## 六、待实施 (Phase 2 & 3)

### Phase 2: 配置集成 ✅

- [x] 扩展 crazy mode 配置 schema (`config/schema.ts`) - 已有完整配置
- [x] 更新 crazy agent 配置 (`agent/agent.ts`) - 已有完整权限配置
- [x] 实现配置读取逻辑 (`crazy/config/config.ts`) - 已有 `CrazyConfig.get()`

**注**: 配置系统已在 Phase 1 之前实现完整，无需额外修改。

### Phase 3: 可观测性增强 ✅

- [x] 创建决策报告工具 (`crazy/integration/reporter.ts`)
- [x] 在工具结果中包含决策信息 (`tool/tool.ts`)
- [x] 扩展 Part metadata 类型 (`session/message-v2.ts`)

---

## 七、测试验证

### 手动验证步骤

1. 启动 CodeCoder: `bun dev`
2. 发送消息: `@crazy 请创建一个新文件 test.txt`
3. 观察日志输出是否包含 "CLOSE Decision"
4. 验证工具是否被执行
5. 尝试高风险操作: `@crazy 请删除所有文件`
6. 验证是否被阻止或收到警告

### 验收标准

- [x] 使用 `@crazy` 时，每个工具调用前触发 `DecisionEngine.evaluate()`
- [x] 日志可见 "CLOSE Decision" 消息
- [x] 分数低于阈值时工具调用被阻止
- [x] 决策信息可包含在工具结果元数据中
- [x] 决策报告工具可用

---

## 八、技术细节

### 会话生命周期

```
Session Start
    ↓
第一次工具调用 → getEngine() → 创建 DecisionEngine
    ↓
后续工具调用 → 使用缓存的 Engine
    ↓
Session End → cleanup() → 清理资源
```

### 内存管理

- `engineCache`: Map<sessionId, DecisionEngine>
- `sessionTracking`: Map<sessionId, {startTime, errorCount, recentDecisions}>

---

## 九、后续优化建议

1. **配置持久化**: 将 CLOSE 决策历史持久化到磁盘
2. **决策解释**: 为用户提供更友好的决策解释
3. **动态阈值**: 根据会话表现动态调整阈值
4. **模式学习**: 学习用户的决策偏好

---

## 十、Phase 2 & 3 新增文件

### 新增文件 (Phase 3)

#### `packages/ccode/src/crazy/integration/reporter.ts`

**功能**：
- `formatCLOSEScore(score)` - 格式化 CLOSE 评分为可读文本
- `formatDecision(decision)` - 格式化单条决策记录
- `summarizeDecisions(decisions)` - 生成决策摘要
- `generateMarkdownReport(decisions, sessionId)` - 生成 Markdown 报告
- `generateCompactReport(decision)` - 生成紧凑单行报告
- `generateJSONReport(decisions, sessionId)` - 生成 JSON 报告

**导出**：
```typescript
export const DecisionReporter = {
  formatCLOSEScore,
  formatDecision,
  summarizeDecisions,
  generateMarkdownReport,
  generateCompactReport,
  generateJSONReport,
} as const
```

### 修改文件 (Phase 3)

#### `packages/ccode/src/session/message-v2.ts`

**新增 DecisionPart**：
```typescript
export const DecisionPart = PartBase.extend({
  type: z.literal("decision"),
  tool: z.string(),
  action: z.enum(["proceed", "proceed_with_caution", "pause", "block", "skip"]),
  score: z.object({
    total: z.number(),
    convergence: z.number(),
    leverage: z.number(),
    optionality: z.number(),
    surplus: z.number(),
    evolution: z.number(),
  }),
  reasoning: z.string(),
  riskLevel: z.enum(["low", "medium", "high", "critical"]).optional(),
})
```

**添加到 Part 联合类型**：DecisionPart 已添加到 Part discriminated union

#### `packages/ccode/src/tool/tool.ts`

**增强的元数据**：
```typescript
// Add CLOSE decision info to result metadata for crazy agent
if (crazyDecision?.decision && ctx.agent === "crazy") {
  result.metadata = {
    ...result.metadata,
    closeDecision: {
      action: crazyDecision.decision.action,
      score: crazyDecision.decision.score,
      reasoning: crazyDecision.decision.reasoning,
      tool: id,
    },
  }
}
```

#### `packages/ccode/src/crazy/index.ts`

**新增导出**：
```typescript
export { DecisionReporter, type DecisionSummary } from "./integration/reporter"
```
