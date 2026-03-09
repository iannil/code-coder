# CodeCoder 文档重构设计：观察者中心架构

> 设计时间: 2026-03-09
> 状态: 已批准

## 1. 背景

CodeCoder 的新定位是"**既能自动，也能手动的 AI 代理观察系统**"。现有文档存在定位不一致的问题：
- README.md 已有 Gear System 概念
- 其他文档仍偏重"执行系统"或"个人工作台"视角
- Observer Network 作为核心特色，在多数文档中反映不够突出

## 2. 设计决策

### 2.1 定位重心
**两者并重**: Observer Network（观察能力）和 Gear System（自动/手动控制）同等重要。

### 2.2 架构层次
```
Gear Control Layer (档位控制)
         │ 控制
         ▼
Observer Network (观察者网络 - 系统核心)
         │ 响应
         ▼
Response Layer (响应层 - Agent 执行)
```

### 2.3 更新策略
**混合策略**: 核心文档重写，其他文档增量修改。

## 3. 文档处理清单

### 3.1 重写文档 (3个)

| 文档 | 主要变更 |
|------|----------|
| `README.md` | 开篇即"观察系统"定位，Gear System 作为控制层 |
| `CLAUDE.md` | 统一定位描述，重组章节顺序 |
| `docs/architecture/ARCHITECTURE.md` | 以观察者架构为核心重构 |

### 3.2 增量修改文档

| 文档 | 变更内容 |
|------|----------|
| `docs/architecture/DESIGN_PHILOSOPHY.md` | 增加观察者视角的哲学解读 |
| `docs/architecture/CORE_CONCEPTS.md` | 调整概念层次，突出观察 |
| `docs/architecture/CCODE_VS_ZERO.md` | 更新定位描述 |
| `docs/architecture/AGENT_FRAMEWORKS.md` | 统一术语 |
| `docs/architecture/SHARED_TYPES.md` | 统一术语 |
| `docs/architecture/ZERO_TRADING_WORKFLOW.md` | 统一术语 |
| `docs/developer-guide.md` | 更新项目介绍部分 |
| `docs/RUNBOOK.md` | 更新描述 |
| `docs/FEATURES.md` | 重组特性列表，观察能力优先 |
| `docs/DEBT.md` | 统一术语 |
| `docs/Skills.md` | 统一术语 |

## 4. 核心术语映射

| 旧术语/描述 | 新术语/描述 |
|------------|------------|
| 个人工作台 | AI 代理观察系统 |
| Agent 系统 | 观察者网络 + 响应代理 |
| 自主执行 | 观察驱动响应 |
| 执行中心 | 观察中心 |
| HITL (人在回路) | Gear 控制 (P/N/D/S/M) |
| 31 个 Agent | 观察者网络 + 31 个响应代理 |

## 5. 新架构图

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Gear Control Layer                              │
│              ┌─────────────────────────────────────────┐                │
│              │     Gear Selector: P  N  D  S  M        │                │
│              ├───────────┬───────────┬───────────┬─────┤                │
│              │  Observe  │  Decide   │    Act    │Gear │                │
│              │   0-100   │   0-100   │   0-100   │ ↑↓  │                │
│              └───────────┴───────────┴───────────┴─────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │ 控制观察深度和响应自动化
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Observer Network                               │
│   ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐              │
│   │ CodeWatch │ │WorldWatch │ │ SelfWatch │ │ MetaWatch │              │
│   │  (代码)   │ │ (世界)    │ │ (自身)    │ │ (元层)    │              │
│   └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘              │
│         └─────────────┴─────────────┴─────────────┘                     │
│                              │ 事件流                                   │
│                    ┌─────────▼─────────┐                                │
│                    │  Consensus Engine │ ← 共识形成                     │
│                    └───────────────────┘                                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │ 驱动响应
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Response Layer                                 │
│   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐                │
│   │Notifier │   │Analyzer │   │Executor │   │Historian│                │
│   │  通知   │   │  分析   │   │  执行   │   │  记录   │                │
│   └─────────┘   └─────────┘   └─────────┘   └─────────┘                │
│                              │                                          │
│                    ┌─────────▼─────────┐                                │
│                    │   31 AI Agents    │ ← 专业响应能力                 │
│                    └───────────────────┘                                │
└─────────────────────────────────────────────────────────────────────────┘
```

## 6. README.md 新结构

```markdown
# CodeCoder

> An AI agent observation system with transmission-like autonomy control

## Overview

CodeCoder is an **observation-centric AI agent system** built on "祝融说"
philosophy. Unlike traditional AI coding assistants, CodeCoder:

1. **Observes first** - Four watchers continuously monitor code, world, self, and meta
2. **Forms consensus** - Consensus engine determines what's happening
3. **Responds controllably** - Gear system lets you control the automation level

## Core Architecture

### Observer Network (观察者网络)
The heart of CodeCoder. Four watchers...

### Gear System (档位控制)
Control autonomy like a car transmission: P/N/D/S/M...

### Response Layer (响应层)
31 specialized AI agents ready to respond...

## ...
```

## 7. 与祝融说哲学的映射

| 哲学概念 | 系统实现 |
|----------|----------|
| 可能性基底 | Observer Network 捕获的原始事件流 |
| 观察即收敛 | Consensus Engine 将可能性坍缩为确定性 |
| 可用余量 | Gear System 保持的控制自由度 |
| 可持续决策 | CLOSE 评估框架 + 风险控制 |

## 8. 验收标准

- [ ] 所有文档的开头描述统一为"AI 代理观察系统"定位
- [ ] Observer Network 在架构文档中处于核心位置
- [ ] Gear System 作为控制层清晰呈现
- [ ] 术语使用一致，无"个人工作台"等旧术语
- [ ] 架构图反映 Gear → Observer → Response 层次
