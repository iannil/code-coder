# Agent 框架文档

> **版本**: 1.0
> **更新时间**: 2026-03-09
> **模块路径**: `packages/ccode/src/agent/frameworks/`

## 概述

Agent 框架是为具有相似结构模式的 Agent 组提取的共享基础设施。这些框架通过提供一致的类型定义、工具函数和报告格式来减少代码重复，同时保持各 Agent 的独立性。

### 设计原则

1. **共享结构，独立行为**: 框架只定义数据结构和工具函数，不影响 Agent 的具体行为逻辑
2. **类型优先**: 所有结构使用 Zod schema 定义，提供运行时验证和 TypeScript 类型
3. **向后兼容**: 框架是增量添加，不破坏现有 Agent 功能
4. **可选使用**: Agent 可以选择性地使用框架中的部分功能

---

## 框架总览

| 框架 | 目标 Agent | 核心能力 |
|------|-----------|---------|
| BaseReviewer | code-reviewer, security-reviewer | 代码分析、发现报告、严重性评估 |
| BaseMarketAnalyst | macro, trader, value-analyst | 市场数据、周期分析、因果链 |
| BasePhilosophyAdvisor | observer, decision | CLOSE 评估、决策分析、祝融说概念 |

---

## BaseReviewer 框架

### 用途

为代码分析类 Agent 提供统一的发现结构、严重性级别和报告格式。

### 核心类型

```typescript
import { BaseReviewer } from "@/agent/frameworks"

// 发现严重性
type Severity = "critical" | "high" | "medium" | "low" | "info"

// 基础发现结构
interface BaseFinding {
  id: string           // 唯一 ID (如 "CR-001")
  severity: Severity
  category: string
  location: Location   // { file, line?, column? }
  title: string
  description: string
  suggestion?: string
  confidence: number   // 0-1
}

// 审查报告
interface BaseReviewReport {
  summary: string
  findings: BaseFinding[]
  metrics: SeverityMetrics
  verdict: "approved" | "approved_with_changes" | "needs_changes" | "not_approved"
  score: number        // 0-100
}
```

### 使用示例

```typescript
import {
  calculateMetrics,
  calculateScore,
  calculateVerdict,
  formatFindingsMarkdown,
} from "@/agent/frameworks"

// 计算指标
const findings: BaseFinding[] = [...]
const metrics = calculateMetrics(findings)
// { critical: 0, high: 2, medium: 5, low: 3, info: 1 }

// 计算分数
const score = calculateScore(findings)
// 75.5

// 计算裁决
const config: BaseReviewerConfig = { autoApproveThreshold: 90 }
const verdict = calculateVerdict(findings, config)
// "needs_changes"

// 格式化为 Markdown
const markdown = formatFindingsMarkdown(findings)
```

### 支持的 Agent

| Agent | 特化类型 | 特有字段 |
|-------|---------|---------|
| code-reviewer | CodeFinding | effort (修复工作量) |
| security-reviewer | SecurityFinding | cweId, cveId, cvssScore |

---

## BaseMarketAnalyst 框架

### 用途

为市场/经济分析类 Agent 提供统一的数据结构、周期分析和报告格式。

### 核心类型

```typescript
import { BaseAnalyst } from "@/agent/frameworks"

// 周期阶段
type CyclePhase = "expansion" | "peak" | "contraction" | "trough" | "recovery"

// 当前状态评估
interface CurrentState {
  cycle: CyclePhase
  position: "early" | "mid" | "late"
  trend: "up" | "flat" | "down"
  confidence: number
  factors: string[]
}

// 市场数据
interface MarketData {
  source: string
  timestamp: Date
  indicators: Indicator[]
  confidence: number
}

// 因果链
interface CausalChain {
  title: string
  links: CausalLink[]  // { from, to, relationship, strength }
}
```

### 使用示例

```typescript
import {
  calculateCycleState,
  formatCausalChainMermaid,
  formatAnalysisMarkdown,
} from "@/agent/frameworks"

// 从指标计算周期状态
const indicators: Indicator[] = [
  { name: "PMI", value: 52.3, interpretation: "bullish" },
  { name: "CPI", value: 2.1, interpretation: "neutral" },
]
const state = calculateCycleState(indicators)
// { cycle: "expansion", position: "mid", trend: "up", confidence: 0.6 }

// 生成因果链 Mermaid 图
const chain: CausalChain = {
  title: "货币政策传导",
  links: [
    { from: "降息", to: "信贷扩张", relationship: "causes", strength: 0.8 },
    { from: "信贷扩张", to: "投资增加", relationship: "enables", strength: 0.7 },
  ],
}
const mermaid = formatCausalChainMermaid(chain)
// ```mermaid
// graph LR
//   subgraph 货币政策传导
//     降息-->信贷扩张
//     信贷扩张-.->投资增加
//   end
// ```
```

### 支持的 Agent

| Agent | 时间范围 | 特有功能 |
|-------|---------|---------|
| macro | medium, long | 经济预测、跨资产相关性 |
| trader | intraday, short | 技术水平、交易计划 |
| value-analyst | medium, long | 内在价值、质量因子 |

---

## BasePhilosophyAdvisor 框架

### 用途

为决策/观察类 Agent 提供祝融说哲学的 CLOSE 框架评估和决策分析结构。

### 核心类型

```typescript
import { BaseAdvisor } from "@/agent/frameworks"

// CLOSE 维度
interface CLOSEDimension {
  score: number      // 0-10
  confidence: number // 0-1
  factors: string[]
  assessment?: string
}

// CLOSE 完整评分
interface CLOSEScore {
  convergence: CLOSEDimension  // 收敛度
  leverage: CLOSEDimension     // 杠杆率
  optionality: CLOSEDimension  // 选择权 (最高权重)
  surplus: CLOSEDimension      // 余量
  evolution: CLOSEDimension    // 进化
  total: number                // 加权总分
  risk: number                 // 风险分数
  confidence: number           // 总置信度
}

// 决策分析
interface DecisionAnalysis {
  title: string
  summary: string
  options: DecisionOption[]
  recommendedOption?: string
  surplusProtection: SurplusProtection[]
  confidence: number
}
```

### 使用示例

```typescript
import {
  quickCLOSEScore,
  buildCLOSEScore,
  formatCLOSEMarkdown,
  formatDecisionMarkdown,
} from "@/agent/frameworks"

// 快速 CLOSE 评分
const score = quickCLOSEScore({
  convergence: 7,
  leverage: 6,
  optionality: 8,  // 高选择权
  surplus: 5,
  evolution: 7,
})
// { total: 6.67, risk: 4.3, confidence: 0.8, ... }

// 格式化为 Markdown (支持中英文)
const markdown = formatCLOSEMarkdown(score, "zh")
// ### CLOSE 评估
// | 维度 | 分数 | 图示 |
// |--------|-------|--------|
// | **收敛度** | 7/10 | [======= ] |
// ...
```

### 哲学概念

框架内置祝融说核心概念的类型和常量：

```typescript
// 可能性空间
interface PossibilitySpace {
  description: string
  entropy: number        // 熵值：可能性数量
  constraints: string[]  // 已知约束
  potentialPaths: string[]
}

// 余量评估
interface MarginAssessment {
  type: "time" | "resources" | "options" | "energy" | "attention" | "relationships"
  level: number         // 0-10
  burnRate: "fast" | "moderate" | "slow" | "stable" | "recovering"
  drains: string[]
  sources: string[]
}

// 哲学原则常量
PHILOSOPHY_PRINCIPLES.zh.possibilitySubstrate
// "可能性基底：宇宙的终极实在是包含一切潜能的无限场域"
```

### 支持的 Agent

| Agent | 聚焦模式 | 输出风格 |
|-------|---------|---------|
| observer | theoretical | reflective |
| decision | practical | actionable |

---

## 与 Observer Network 的关系

BaseAdvisor 框架与 Observer Network 的 `close-evaluator.ts` 互补而非重复：

| 组件 | 用途 | 输入 | 输出 |
|------|------|------|------|
| Observer CLOSEEvaluator | 系统模式决策 | ConsensusSnapshot | ModeDecision (AUTO/HYBRID/MANUAL) |
| BaseAdvisor CLOSEScore | 用户决策支持 | 用户问题 | DecisionAnalysis (选项评估) |

Observer 的 CLOSE 用于自动化系统行为，BaseAdvisor 的 CLOSE 用于辅助人类决策。

---

## 文件结构

```
packages/ccode/src/agent/frameworks/
├── index.ts           # 统一导出
├── base-reviewer.ts   # 代码审查框架
├── base-analyst.ts    # 市场分析框架
└── base-advisor.ts    # 决策顾问框架
```

## 导入方式

```typescript
// 命名空间导入 (推荐)
import { BaseReviewer, BaseAnalyst, BaseAdvisor } from "@/agent/frameworks"

// 直接导入特定类型/函数
import {
  type CLOSEScore,
  quickCLOSEScore,
  formatCLOSEMarkdown,
} from "@/agent/frameworks"
```

---

## 未来扩展

### 潜在的新框架

| 框架候选 | 目标 Agent | 共享模式 |
|---------|-----------|---------|
| BaseContentCreator | writer, expander, proofreader | 内容结构、质量检查 |
| BaseReverseEngineer | code-reverse, jar-code-reverse | 逆向分析、协议解码 |
| BaseProductAdvisor | picker, miniproduct, feasibility-assess | 产品评估、可行性分析 |

### 贡献指南

添加新框架时：

1. 在 `frameworks/` 目录创建 `base-{name}.ts`
2. 使用 Zod schema 定义所有类型
3. 提供实用工具函数和 Markdown 格式化
4. 在 `index.ts` 中导出
5. 更新本文档

---

## 相关文档

- [Agent 能力矩阵](./AGENT_CAPABILITY_MATRIX.md)
- [Observer Network 架构](../CLAUDE.md#观察者网络-observer-network)
- [祝融说哲学框架](../../packages/ccode/src/agent/prompt/decision.txt)
