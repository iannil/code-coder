# Autonomous Agent 在 Thinking 模式下的截断问题修复

**日期**: 2026-02-13
**状态**: 已完成

## 问题概述

Autonomous agent 在生成代码时遇到截断问题，特别是在使用启用了 thinking 模式的模型变体（如 "high" variant）时。

### 问题场景

当 autonomous agent 同时启用：
1. `agent.options.maxOutputTokens = 128_000`
2. thinking 模式（如 `high` variant: `budgetTokens = 16_000`）

计算流程导致实际输出限制被意外减少，影响代码生成的完整性。

---

## 根本原因分析

### Token 计算逻辑

**文件**: `packages/ccode/src/provider/transform.ts`

原始 `maxOutputTokens` 函数的问题：

```typescript
// 原始逻辑
if (enabled && budgetTokens > 0) {
  if (budgetTokens + standardLimit <= modelCap) {
    return standardLimit  // ← 这里没问题
  }
  return modelCap - budgetTokens  // ← 问题：可能返回过小的值
}
```

### 问题场景

**场景 1**: agent 配置 128k，模型上限 200k，thinking budget 16k
```
standardLimit = Math.min(200_000, 128_000) = 128_000
budgetTokens = 16_000

128_000 + 16_000 = 144_000 <= 200_000
所以返回 128_000 ✓ 正确
```

**场景 2**: 模型自带小限制（如 `limit.output = 8192`），但 agent 配置 128k
```
standardLimit = Math.min(200_000, 8_192) = 8_192  // ← 被模型限制约束
budgetTokens = 16_000

8_192 + 16_000 = 24_192 <= 200_000
所以返回 8_192  // ← 实际上可能不符合用户预期
```

**场景 3**: Thinking 选项被显式设置但应该被禁用
```
// 当 agent 配置 thinking: { type: "disabled" }
// 但模型配置有默认的 thinking budget
// 原逻辑没有正确处理 "disabled" 类型
```

---

## 修复方案

### 方案 1：禁用 Autonomous Agent 的 Thinking 模式（P0 - 已实施）

**修改文件**: `packages/ccode/src/agent/agent.ts`

```typescript
autonomous: {
  name: "autonomous",
  description: "自主模式 - 完全自主的执行代理...",
  options: {
    maxOutputTokens: 128_000,
    // 禁用 thinking 模式以防止在大 maxOutputTokens 时出现截断
    // Thinking 模式会减少可用的 output tokens 数量（budgetTokens）
    // 如需启用，可设置环境变量 CCODE_AUTONOMOUS_THINKING=true
    thinking: { type: "disabled" },
  },
  // ...
}
```

### 方案 2：改进 maxOutputTokens 计算逻辑（P1 - 已实施）

**修改文件**: `packages/ccode/src/provider/transform.ts`

```typescript
export function maxOutputTokens(
  npm: string,
  options: Record<string, any>,
  modelLimit: number,
  globalLimit: number,
): number {
  const modelCap = modelLimit || globalLimit
  const standardLimit = Math.min(modelCap, globalLimit)

  if (npm === "@ai-sdk/anthropic" || npm === "@ai-sdk/google-vertex/anthropic") {
    const thinking = options?.["thinking"]

    // 检查 thinking 是否显式禁用 - 返回标准限制
    if (thinking?.["type"] === "disabled") {
      return standardLimit
    }

    const budgetTokens = typeof thinking?.["budgetTokens"] === "number"
      ? thinking["budgetTokens"] : 0
    const enabled = thinking?.["type"] === "enabled"

    if (enabled && budgetTokens > 0) {
      const availableForOutput = modelCap - budgetTokens

      // 如果标准限制 + thinking budget 适合模型上限，保留用户/agent 配置
      if (standardLimit + budgetTokens <= modelCap) {
        return standardLimit
      }

      // 否则计算可用空间，确保至少 80% 的可用空间
      const standardOutput = Math.min(standardLimit, availableForOutput)
      const minimumOutput = Math.min(modelCap * 0.8, availableForOutput)
      return Math.max(standardOutput, minimumOutput)
    }
  }

  return standardLimit
}
```

**改进点**:
1. 正确处理 `thinking.type === "disabled"` 情况
2. 当 thinking 启用时，优先保留 agent/user 的 `maxOutputTokens` 配置
3. 添加 80% 最小输出保护，防止极端情况下的过小输出

### 方案 3：添加测试（P1 - 已实施）

**修改文件**: `packages/ccode/test/provider/transform.test.ts`

新增测试用例：
- `thinking type is disabled` - 验证禁用时的行为
- `autonomous agent scenario` - 验证 128k maxOutput with 16k budgetTokens
- `ensures minimum 80%` - 验证边界情况的最小输出保护
- `google-vertex/anthropic` - 验证兼容提供商

---

## 修改文件清单

| 文件 | 修改类型 | 优先级 |
|------|----------|--------|
| `packages/ccode/src/agent/agent.ts` | 配置修改：禁用 thinking | P0 |
| `packages/ccode/src/provider/transform.ts` | 逻辑优化：改进计算 | P1 |
| `packages/ccode/test/provider/transform.test.ts` | 测试更新：新增用例 | P1 |
| `docs/progress/2026-02-13-autonomous-truncation-fix.md` | 文档记录 | P2 |

---

## 测试验证

### 单元测试

```bash
cd packages/ccode && bun test test/provider/transform.test.ts
```

结果：85 个测试全部通过

### 集成测试

```bash
cd packages/ccode && bun test test/integration/autonomous-mode.test.tsx
```

结果：34 个测试全部通过

### 手动测试

1. 启动 CodeCoder：
   ```bash
   bun dev
   ```

2. 使用 autonomous agent 生成代码：
   ```
   > @autonomous 生成一个完整的 REST API 服务，包含 CRUD 操作和测试
   ```

3. 验证输出完整，无截断

---

## 环境变量控制

用户可以通过环境变量覆盖 autonomous agent 的 thinking 设置：

```bash
# 启用 thinking 模式（默认禁用）
CCODE_AUTONOMOUS_THINKING=true bun dev
```

或在配置文件中设置：

```json
{
  "agent": {
    "autonomous": {
      "options": {
        "thinking": { "type": "enabled" }
      }
    }
  }
}
```

---

## 后续可选改进

### 方案 4：输出续写机制（未实施）

**新增文件**: `packages/ccode/src/autonomous/continuation.ts`

```typescript
export interface ContinuationConfig {
  enabled: boolean
  maxContinuations: number
  triggerThreshold: number  // 当剩余 tokens 少于此值时触发续写
}

export async function continueOutput(
  sessionId: string,
  context: string,
  originalMaxTokens: number,
): Promise<string | null> {
  // 检测输出是否被截断
  // 如果被截断，使用相同上下文继续生成
}
```

**优点**:
- 完全解决截断问题
- 适用于任何模型

**缺点**:
- 增加复杂度
- 需要多次 API 调用
- 成本增加

---

## 总结

本次修复采用综合方案：

1. **短期 (P0)**: 为 autonomous agent 默认禁用 thinking 模式
2. **长期 (P1)**: 改进 `maxOutputTokens` 计算逻辑

用户现在可以：
- 使用 autonomous agent 而不用担心 thinking 模式导致的截断
- 通过环境变量 `CCODE_AUTONOMOUS_THINKING` 重新启用 thinking 模式
- 受益于改进的 token 计算逻辑，更好的输出保护
