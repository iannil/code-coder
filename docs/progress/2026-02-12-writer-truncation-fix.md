# Writer Agent 截断问题修复

**日期**: 2026-02-12
**状态**: 已完成

## 问题概述

Writer Agent 用于长篇内容（20k+ 字）创作，采用章节式写作模式：
- Phase 1: 大纲生成
- Phase 2: 章节编写
- Phase 3: 章节审查

**当前问题：章节输出总是被截断**

---

## 根本原因

### 1. 输出 Token 限制过小

**文件：** `packages/ccode/src/session/llm.ts:30`

```typescript
export const OUTPUT_TOKEN_MAX = Flag.CCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX || 32_000
```

**问题：**
- 默认限制为 32,000 tokens
- 对于长章节（目标 3000-5000 中文字），需要约 5,000-10,000 tokens
- 加上摘要和元数据，单次请求可能超过此限制

### 2. 上下文预算与实际限制不匹配

**文件：** `packages/ccode/src/document/context.ts:7-17`

```typescript
const DEFAULT_BUDGET: DocumentSchema.ContextBudget = {
  totalTokens: 100000,
  reservedOutputTokens: 40000,  // 预留 40k
  // ...
}
```

**矛盾：**
- context budget 预留了 40,000 tokens 用于输出
- 但实际 OUTPUT_TOKEN_MAX 只有 32,000
- 存在 8,000 tokens 的差距

### 3. 模型实际输出限制

不同模型的输出限制差异巨大：

| 模型 | 最大输出 | Writer 可用 |
|------|----------|-------------|
| Claude 3.5 Sonnet | 8,192 | 8,192 (受 32k 限制) |
| Claude 3 Opus | 4,096 | 4,096 |
| GPT-4o | 4,096 | 4,096 |
| Claude 3.6 Sonnet | 200,000 | 32,000 (受限制) |

**关键发现：** Claude 3.6 等新模型支持更大的输出，但被 32k 的硬限制约束。

---

## 实施的改进

### 阶段 1：动态输出限制（已实现）

为 writer 和 proofreader agent 使用专门的输出限制，而非全局固定值。

**修改的文件：**

1. **`packages/ccode/src/session/llm.ts`**
   ```typescript
   // Writer agent needs larger output tokens for long-form content
   // Allow agent.options.maxOutputTokens to override default
   const agentMaxTokens = input.agent.options?.maxOutputTokens ?? OUTPUT_TOKEN_MAX
   ```

2. **`packages/ccode/src/agent/agent.ts`**
   ```typescript
   writer: {
     options: { maxOutputTokens: 128_000 },
     // ...
   },
   proofreader: {
     options: { maxOutputTokens: 128_000 },
     // ...
   },
   ```

3. **`packages/ccode/src/document/context.ts`**
   ```typescript
   const DEFAULT_BUDGET: DocumentSchema.ContextBudget = {
     totalTokens: 200000,        // 更新为 200k
     reservedOutputTokens: 128000,  // 增加预留
     // ...
   }
   ```

**优点：**
- 不影响其他 agent
- 可以根据模型动态调整
- 充分利用大上下文模型能力

### 阶段 2：截断检测（已实现）

在 writer 命令执行逻辑中添加截断检测功能。

**修改的文件：**

1. **`packages/ccode/src/document/writer.ts`**
   ```typescript
   export function detectTruncation(
     content: string,
     estimatedWords: number
   ): string | undefined {
     // 检测截断标记
     // 检测句子突然结束
     // 检测字数偏差（低于目标50%）
   }
   ```

2. **`packages/ccode/src/cli/cmd/document.ts`**
   - 在章节写入后调用检测函数
   - 输出警告信息

---

## 修改文件清单

| 文件 | 修改类型 | 优先级 |
|------|----------|--------|
| `packages/ccode/src/session/llm.ts` | 修改 maxOutputTokens 逻辑 | P0 |
| `packages/ccode/src/document/context.ts` | 更新 DEFAULT_BUDGET | P0 |
| `packages/ccode/src/agent/agent.ts` | 添加 agent 级别配置 | P1 |
| `packages/ccode/src/document/writer.ts` | 添加截断检测 | P1 |
| `packages/ccode/src/cli/cmd/document.ts` | 使用截断检测 | P1 |

---

## 验证方法

1. **单元测试**
   - 测试 maxOutputTokens 计算逻辑
   - 测试不同 agent 的限制

2. **集成测试**
   - 使用 writer agent 生成 5000 字章节
   - 验证完整输出无截断

3. **手动测试**
   ```bash
   CCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX=128000 bun dev
   # 然后使用 writer agent 生成章节
   ```

---

## 后续可选改进

### 阶段 3：自动续写（可选）

当检测到输出可能被截断时，自动续写。

**实施步骤：**

1. **检测截断**
   - 监控流式输出的 finishReason
   - 如果是 'length'，说明被截断

2. **自动续写**
   - 将已写内容作为上下文
   - 提示模型从断点继续
   - 重复直到完成

3. **拼接内容**
   - 合并多次输出
   - 确保边界平滑

**优点：**
- 兼容所有模型
- 可控性强
- 适用于任何长内容场景

**缺点：**
- 需要多次 API 调用
- 成本增加
- 接缝处理复杂

---

## 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 输入+输出超出上下文窗口 | 高 | 动态计算实际可用输入 |
| 某些模型不支持大输出 | 中 | 保持 modelCap 限制 |
| 成本增加 | 低 | 由用户选择是否启用 |

---

## 总结

本次修复主要完成了阶段 1 和阶段 2：

1. **阶段 1**：为 writer 和 proofreader agent 设置了 128k 的最大输出 token
2. **阶段 2**：添加了截断检测功能，当内容可能被截断时警告用户

用户现在可以通过环境变量 `CCODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX` 或在配置中设置 `agent.options.maxOutputTokens` 来控制特定 agent 的输出限制。
