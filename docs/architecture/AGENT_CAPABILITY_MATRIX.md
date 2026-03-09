# Agent 能力矩阵

> **版本**: 1.3
> **更新时间**: 2026-03-09
> **Agent 总数**: 29

## 总览

CodeCoder 包含 29 个 Agent，分为主模式、子代理和系统隐藏三类。其中 14 个 Agent 具备 Observer Network 集成能力，可参与观察者网络的共识形成。

### 共享框架

为减少代码重复，具有相似结构的 Agent 组共享基础框架：

| 框架 | Agent | 共享能力 |
|------|-------|---------|
| BaseReviewer | code-reviewer, security-reviewer | 发现结构、严重性、报告格式 |
| BaseMarketAnalyst | macro, trader, value-analyst | 市场数据、周期分析、因果链 |
| BasePhilosophyAdvisor | observer, decision | CLOSE 评估、决策分析 |

详见 [Agent 框架文档](./AGENT_FRAMEWORKS.md)。

### 模式分类

| 模式 | 数量 | 说明 |
|------|------|------|
| primary | 5 | 主要开发模式，可直接使用 |
| subagent | 21 | 子代理，由主 Agent 调用 |
| hidden | 3 | 系统内部使用，不对用户显示 |

### Observer Network 集成

| 观察者类型 | 已集成 Agent | 说明 |
|------------|-------------|------|
| CodeWatch | explore, architect, feasibility-assess | 代码库扫描、架构分析、可行性评估 |
| WorldWatch | macro, trader, value-analyst, picker | 市场数据、宏观经济、价值分析、选品观察 |
| SelfWatch | code-reviewer, security-reviewer, decision, tdd-guide, verifier, autonomous | 代码质量、安全评估、决策状态、测试质量、验证结果、自主执行监控 |
| MetaWatch | observer | 观察者系统自省 |

---

## 完整能力矩阵

### 主模式 Agent (5)

| Agent | 描述 | Observer | Temperature | 特殊配置 |
|-------|------|----------|-------------|----------|
| **build** | 默认开发模式 | - | default | native, plan_enter |
| **plan** | 实现规划模式 | - | default | maxOutputTokens: 128k |
| **writer** | 长文写作（20k+ 字） | - | 0.7 | maxOutputTokens: 128k, thinking: disabled |
| **autonomous** | 完全自主执行代理（CLOSE 框架） | SelfWatch | 0.6 | maxOutputTokens: 128k, color: magenta |

### 工程质量 Agent (6)

| Agent | 描述 | Observer | Temperature | 框架 | 特殊配置 |
|-------|------|----------|-------------|------|----------|
| **general** | 通用多步骤任务执行 | - | default | - | autoApprove: Read/Glob/Grep/LS |
| **explore** | 快速代码库探索 | CodeWatch | default | - | autoApprove: Read/Glob/Grep/LS/WebFetch/WebSearch |
| **code-reviewer** | 代码质量审查 | SelfWatch | default | BaseReviewer | - |
| **security-reviewer** | 安全漏洞分析 | SelfWatch | default | BaseReviewer | - |
| **tdd-guide** | TDD 方法论指导 | SelfWatch | default | - | - |
| **architect** | 系统架构设计 | CodeWatch | default | - | - |

### 逆向工程 Agent (2)

| Agent | 描述 | Observer | Temperature | 特殊配置 |
|-------|------|----------|-------------|----------|
| **code-reverse** | 网站逆向工程 | - | 0.3 | color: cyan |
| **jar-code-reverse** | JAR 逆向工程 | - | 0.3 | color: magenta |

### 内容创作 Agent (3)

| Agent | 描述 | Observer | Temperature | 特殊配置 |
|-------|------|----------|-------------|----------|
| **expander** | 统一内容扩展（支持 fiction/nonfiction 自动检测或 [DOMAIN:tag] 显式指定） | - | 0.7 | maxOutputTokens: 128k |
| **proofreader** | PROOF 框架校对 | - | 0.3 | maxOutputTokens: 128k |
| **verifier** | 代码验证（构建/测试/覆盖率） | SelfWatch | 0.1 | - |

### 祝融说系列 Agent (8)

| Agent | 描述 | Observer | Temperature | 框架 | 特殊配置 |
|-------|------|----------|-------------|------|----------|
| **observer** | 观察者理论分析 | MetaWatch | 0.7 | BaseAdvisor | reportToMeta: false |
| **decision** | CLOSE 五维决策框架 | SelfWatch | 0.6 | BaseAdvisor | - |
| **macro** | 宏观经济分析（18 章体系） | WorldWatch | 0.5 | BaseAnalyst | - |
| **trader** | 超短线交易技术分析 | WorldWatch | 0.5 | BaseAnalyst | - |
| **picker** | 爆品选品方法论 | WorldWatch | 0.6 | - | - |
| **miniproduct** | 极小产品开发指导 | - | 0.6 | - | - |
| **ai-engineer** | AI 工程师实战教程 | - | 0.5 | - | - |
| **value-analyst** | 价值分析（观察者建构论） | WorldWatch | 0.5 | BaseAnalyst | - |

### 产品与可行性 Agent (2)

| Agent | 描述 | Observer | Temperature | 特殊配置 |
|-------|------|----------|-------------|----------|
| **prd-generator** | PRD 文档生成 | - | 0.5 | maxOutputTokens: 64k, color: blue |
| **feasibility-assess** | 技术可行性评估 | CodeWatch | 0.3 | color: yellow |

### 其他 Agent (1)

| Agent | 描述 | Observer | Temperature | 特殊配置 |
|-------|------|----------|-------------|----------|
| **synton-assistant** | SYNTON-DB 使用助手 | - | 0.5 | - |

### 系统隐藏 Agent (3)

| Agent | 描述 | 用途 |
|-------|------|------|
| **compaction** | 会话压缩 | 自动上下文管理 |
| **title** | 标题生成 | 会话标题生成 |
| **summary** | 摘要生成 | 会话摘要生成 |

---

## Observer 集成详情

### 已集成 Agent (14)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Observer Network                                    │
│                                                                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │ CodeWatch  │  │ WorldWatch │  │ SelfWatch  │  │ MetaWatch  │            │
│  │            │  │            │  │            │  │            │            │
│  │  explore   │  │   macro    │  │   code-    │  │  observer  │            │
│  │  architect │  │   trader   │  │  reviewer  │  │            │            │
│  │feasibility-│  │   value-   │  │ security-  │  │            │            │
│  │  assess    │  │  analyst   │  │  reviewer  │  │            │            │
│  │            │  │   picker   │  │  decision  │  │            │            │
│  │            │  │            │  │  tdd-guide │  │            │            │
│  │            │  │            │  │  verifier  │  │            │            │
│  │            │  │            │  │ autonomous │  │            │            │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 集成能力说明

| Agent | canWatch | contributeToConsensus | reportToMeta |
|-------|----------|----------------------|--------------|
| explore | code | true | true |
| architect | code | true | true |
| feasibility-assess | code | true | true |
| code-reviewer | self | true | true |
| security-reviewer | self | true | true |
| decision | self | true | true |
| tdd-guide | self | true | true |
| verifier | self | true | true |
| autonomous | self | true | true |
| observer | meta | true | false |
| macro | world | true | true |
| trader | world | true | true |
| value-analyst | world | true | true |
| picker | world | true | true |

### 未集成 Agent (15)

以下 Agent 尚未配置 Observer Network 能力，但可以通过配置添加：

- **主模式**: build, plan, writer
- **工程质量**: general
- **逆向工程**: code-reverse, jar-code-reverse
- **内容创作**: expander, proofreader
- **祝融说**: miniproduct, ai-engineer
- **产品**: prd-generator
- **其他**: synton-assistant

---

## 功能重叠分析

### 高重叠对 (>70%)

| Agent 组 | 重叠内容 | 状态 |
|----------|----------|------|
| ~~expander / expander-fiction / expander-nonfiction~~ | 核心扩展逻辑相似，仅 temperature 和领域提示不同 | ✅ **已合并** (2026-03-09) |

### 中等重叠对 (50%-70%)

| Agent 组 | 重叠内容 | 建议 |
|----------|----------|------|
| code-reviewer / security-reviewer | 都进行代码分析，但关注点不同 | **保持独立**，可共享基础分析框架 |
| macro / trader | 都关注市场数据，但分析周期不同 | **保持独立**，领域差异大 |

### 低重叠对 (<50%)

- observer / decision - 哲学框架不同
- explore / general - 工具权限不同
- verifier / tdd-guide - 验证 vs 开发流程

---

## Agent 合并评估报告

### 评估标准

| 重叠度 | 判定 | 行动 |
|--------|------|------|
| > 70% | 强烈建议合并 | 立即实施合并，通过参数区分行为 |
| 50%-70% | 考虑参数化 | 评估是否共享基础代码，保持独立界面 |
| < 50% | 保持独立 | 无需合并，各自优化 |

### 详细评估结果

#### 1. Expander 系列 ✅ 已完成 (2026-03-09)

**合并前状态**:
```
expander          → temperature: 0.7, maxOutputTokens: 128k
expander-fiction  → temperature: 0.8, maxOutputTokens: 128k (已移除)
expander-nonfiction → temperature: 0.6, maxOutputTokens: 128k (已移除)
```

**合并后状态**:
```typescript
expander: {
  name: "expander",
  description: "Unified content expansion specialist supporting fiction, non-fiction, and general content. Auto-detects domain or accepts explicit [DOMAIN:fiction|nonfiction] tag.",
  options: {
    maxOutputTokens: 128_000,
    thinking: { type: "disabled" },
  },
  temperature: 0.7,  // 平衡创意与精确
}
```

**实现方式**:
- 统一 prompt 包含领域检测逻辑
- 自动识别 fiction/nonfiction 关键词
- 支持显式 `[DOMAIN:fiction]` 或 `[DOMAIN:nonfiction]` 标签覆盖
- 原 prompt 文件 (expander-fiction.txt, expander-nonfiction.txt) 保留作为备份

**收益**:
- Agent 数量: 31 → 29 (-2)
- 简化用户选择
- 统一扩展逻辑维护

---

#### 2. Code-Reviewer / Security-Reviewer (保持独立)

**当前状态**:
```
code-reviewer     → SelfWatch, default temperature
security-reviewer → SelfWatch, default temperature
```

**重叠度**: ~55%
- 都分析代码质量
- 但关注点完全不同（质量 vs 安全）

**建议**: 保持独立，但共享基础分析框架

**优化方案**:
```typescript
// 创建共享的 BaseReviewer trait
interface BaseReviewer {
  analyzeFile(path: string): Promise<Finding[]>
  formatReport(findings: Finding[]): string
}

// code-reviewer 继承并特化
class CodeReviewer extends BaseReviewer {
  // 关注: 可读性、模式、性能
}

// security-reviewer 继承并特化
class SecurityReviewer extends BaseReviewer {
  // 关注: OWASP Top 10, 输入验证, 密钥泄露
}
```

---

#### 3. Macro / Trader (保持独立)

**当前状态**:
```
macro  → WorldWatch, temperature: 0.5, 宏观经济分析
trader → WorldWatch, temperature: 0.5, 超短线交易技术
```

**重叠度**: ~45%
- 都观察市场数据
- 但分析周期和方法论完全不同

**建议**: 保持独立

**理由**:
- 宏观分析 (macro): 关注 GDP、货币政策、长周期趋势
- 交易分析 (trader): 关注 K 线模式、情绪周期、短周期技术

---

#### 4. General / Explore (保持独立)

**当前状态**:
```
general → 通用任务执行, autoApprove: Read/Glob/Grep/LS
explore → 代码库探索, autoApprove: 更多工具, CodeWatch
```

**重叠度**: ~40%
- general 用于多步骤任务
- explore 专注代码库探索

**建议**: 保持独立

---

### 实施优先级

| 优先级 | Agent 组 | 行动 | 状态 |
|--------|----------|------|------|
| P1 | expander 系列 | 合并为单一 agent + domain 参数 | ✅ 已完成 |
| P2 | reviewer 系列 | 提取共享基础框架 | ✅ **已完成** (2026-03-09) |
| P2 | analyst 系列 | 提取 BaseMarketAnalyst 框架 | ✅ **已完成** (2026-03-09) |
| P2 | advisor 系列 | 提取 BasePhilosophyAdvisor 框架 | ✅ **已完成** (2026-03-09) |
| P3 | - | 无进一步合并需求 | - |

### 合并后的 Agent 数量

| 分类 | 合并前 | 合并后 | 变化 |
|------|--------|--------|------|
| 总计 | 31 | 29 | -2 |
| 内容创作 | 5 | 3 | -2 |
| 其他 | 不变 | 不变 | 0 |

---

## 配置扩展指南

### 为 Agent 添加 Observer 能力

在 `~/.codecoder/config.json` 中配置：

```json
{
  "agent": {
    "your-agent": {
      "observerCapability": {
        "canWatch": ["code", "self"],
        "contributeToConsensus": true,
        "reportToMeta": true
      }
    }
  }
}
```

### 自定义 Agent 参与 Observer Network

```typescript
// 在 agent 配置中添加
{
  name: "custom-agent",
  mode: "subagent",
  observerCapability: {
    canWatch: ["world"], // 可以是 code, world, self, meta
    contributeToConsensus: true,
    reportToMeta: true,
  },
}
```

---

## 相关文档

- [Agent 框架文档](./AGENT_FRAMEWORKS.md)
- [Observer Network 架构](../CLAUDE.md#观察者网络-observer-network)
- [Agent 3-Mode 系统](../CLAUDE.md#agent-3-mode-系统)
- [Agent 源码](../../packages/ccode/src/agent/agent.ts)
- [框架源码](../../packages/ccode/src/agent/frameworks/)
- [Observer 集成](../../packages/ccode/src/observer/agent-registry.ts)
