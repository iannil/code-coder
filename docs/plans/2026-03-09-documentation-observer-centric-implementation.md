# Documentation Observer-Centric Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor all CodeCoder documentation to reflect the "AI agent observation system with transmission-like autonomy control" positioning.

**Architecture:** Observer Network as core, Gear System as control layer, Response Layer (Agents) as execution. The narrative order is Gear → Observer → Response.

**Tech Stack:** Markdown documentation, ASCII diagrams

**Reference:** See `docs/plans/2026-03-09-documentation-observer-centric-design.md` for detailed design decisions.

---

## Phase 1: Core Document Rewrites (3 documents)

### Task 1: Rewrite README.md Overview Section

**Files:**
- Modify: `README.md:1-20`

**Step 1: Update the opening description**

Change line 3 from:
```markdown
> An AI agent observation system with transmission-like autonomy control — from fully automatic to fully manual.
```

To:
```markdown
> An observation-centric AI agent system — observe first, control always.
```

**Step 2: Rewrite the Overview section (lines 11-16)**

Replace the existing Overview with:

```markdown
## Overview

CodeCoder is an **observation-centric AI agent system** built on "祝融说" (Zhu Rong Philosophy). Unlike traditional AI assistants that wait for commands, CodeCoder:

1. **Observes continuously** — Four watchers monitor code, world, self, and meta-layer
2. **Forms consensus** — Consensus engine determines what's happening and what matters
3. **Responds controllably** — Gear System (P/N/D/S/M) lets you dial in exactly how autonomous the response should be

The core insight: **Observation before action, control over automation.**
```

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: update README overview to observation-centric positioning"
```

---

### Task 2: Reorder README.md Core Architecture Section

**Files:**
- Modify: `README.md:69-217`

**Step 1: Move Observer Network section before Gear System**

Reorder the README structure to:
1. Overview (already updated)
2. **Observer Network** (move up, expand)
3. **Gear System** (move down, as control layer)
4. Response Layer / Agent System
5. Features
6. Quick Start
7. Development
8. Design Philosophy

**Step 2: Update the Features list (line 69-77)**

Replace with:

```markdown
## Features

**Observation Layer:**
- **Observer Network** — Four watchers (Code, World, Self, Meta) with consensus engine
- **Continuous monitoring** — Always-on observation with configurable depth

**Control Layer:**
- **Gear System** — P/N/D/S/M transmission-like autonomy control
- **Three dials** — Fine-grained control over Observe/Decide/Act dimensions

**Response Layer:**
- **31 AI Agents** — Specialized responders organized in 3 modes (build, writer, decision)
- **Multi-provider support** — Claude, GPT, Gemini, Ollama, and 20+ providers

**Infrastructure:**
- **Dual-language architecture** — TypeScript for intelligence, Rust for security boundaries
- **Markdown-based memory** — Transparent, Git-friendly knowledge system
- **Multi-platform IM** — Telegram, Discord, Slack, Feishu, Email
```

**Step 3: Update Architecture diagram section (line 119-182)**

Ensure the diagram shows: Gear Control → Observer Network → Response Layer

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: reorder README to observation-first structure"
```

---

### Task 3: Rewrite CLAUDE.md Project Overview

**Files:**
- Modify: `CLAUDE.md:1-25`

**Step 1: Update the first line**

Change:
```markdown
本文件为 Claude Code (claude.ai/code) 在此代码库中工作时提供指导。
```

Keep as is (this is fine).

**Step 2: Rewrite 项目概述 section (lines 5-25)**

Replace with:

```markdown
## 项目概述

CodeCoder 是一个**以观察为中心的 AI 代理系统**，可实现从完全自动到完全手动的自主控制。

核心特性：
- **观察者网络 (Observer Network)** — 四大观察者持续监控代码、世界、自身和元层
- **档位控制 (Gear System)** — P/N/D/S/M 五档 + 三旋钮精确控制自动化程度
- **响应代理 (Response Agents)** — 31 个专业 AI Agent 作为响应能力

使用 Turborepo 和 Bun 构建的 monorepo，包含 TypeScript (packages/ccode) 和 Rust (services/zero-*) 双语言架构。

### 架构层次

```
Gear Control Layer (档位控制)
         │ 控制观察深度和响应自动化
         ▼
Observer Network (观察者网络)
         │ 形成共识后驱动响应
         ▼
Response Layer (响应层 - 31 个 Agent)
```

### 核心定位

| 层级 | 实现 |
|------|------|
| **观察层** | CodeWatch, WorldWatch, SelfWatch, MetaWatch → Consensus Engine |
| **控制层** | Gear System (P/N/D/S/M) + Three Dials (Observe/Decide/Act) |
| **响应层** | 31 个专业 Agent：工程、领域、思维三大类 |

### 哲学框架

本项目内置基于"祝融说"的决策与认知系统，核心理念：

- **可能性基底** → Observer Network 捕获的原始事件流
- **观察即收敛** → Consensus Engine 将可能性坍缩为确定性
- **可用余量** → Gear System 保持的控制自由度
- **可持续决策 > 最优决策** → CLOSE 评估框架 + 风险控制
```

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: rewrite CLAUDE.md project overview to observation-centric"
```

---

### Task 4: Reorganize CLAUDE.md Structure

**Files:**
- Modify: `CLAUDE.md` (full file)

**Step 1: Move Observer Network section up**

Current order:
1. 项目概述
2. 项目指南
3. 开发命令
4. 架构
5. Agent 架构
6. **观察者网络** (line 409)
7. 代码风格指南

New order:
1. 项目概述 (updated)
2. **观察者网络** (move up, right after overview)
3. **档位控制系统** (add new section)
4. Agent 架构 (rename to 响应代理系统)
5. 架构
6. 项目指南
7. 开发命令
8. 代码风格指南

**Step 2: Add a new 档位控制系统 section after Observer Network**

```markdown
## 档位控制系统 (Gear System)

Gear System 控制 Observer Network 的观察深度和 Response Layer 的自动化程度。

### 五档控制

| 档位 | 模式 | Observe | Decide | Act | 适用场景 |
|------|------|---------|--------|-----|----------|
| **P** | Park | 0% | 0% | 0% | 系统休眠，无资源消耗 |
| **N** | Neutral | 50% | 0% | 0% | 纯观察，不干预 |
| **D** | Drive | 70% | 60% | 40% | 日常平衡模式（默认） |
| **S** | Sport | 90% | 80% | 70% | 全自主模式 |
| **M** | Manual | 自定义 | 自定义 | 自定义 | 三旋钮精细控制 |

### 三旋钮 (Manual 模式)

- **Observe (0-100%)**: 观察深度 — 0% 被动等待，100% 主动扫描
- **Decide (0-100%)**: 决策自主 — 0% 仅建议，100% 自主决策
- **Act (0-100%)**: 执行自主 — 0% 等待确认，100% 立即执行

### 命令行使用

```bash
bun dev              # 默认 D 档
bun dev --gear S     # S 档（高自主）
bun dev --gear N     # N 档（纯观察）
bun dev --gear M --observe 80 --decide 30 --act 10  # 自定义
```
```

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: reorganize CLAUDE.md with observation-first structure"
```

---

### Task 5: Rewrite docs/architecture/ARCHITECTURE.md Opening

**Files:**
- Modify: `docs/architecture/ARCHITECTURE.md:1-100`

**Step 1: Update the title and opening**

Change from:
```markdown
# CodeCoder 系统架构

> 生成时间: 2026-03-08

## 1. 项目概述

CodeCoder 是一个融合工程能力与决策智慧的个人工作台...
```

To:
```markdown
# CodeCoder 系统架构

> 生成时间: 2026-03-09
> 定位: 以观察为中心的 AI 代理系统

## 1. 项目概述

CodeCoder 是一个**以观察为中心的 AI 代理系统**，实现从完全自动到完全手动的自主控制。

### 核心架构层次

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Gear Control Layer                              │
│                        (档位控制 - 用户界面)                             │
│              ┌─────────────────────────────────────────┐                │
│              │     Gear Selector: P  N  D  S  M        │                │
│              └─────────────────────────────────────────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                                 │ 控制
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Observer Network                               │
│                        (观察者网络 - 系统核心)                           │
│   ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐              │
│   │ CodeWatch │ │WorldWatch │ │ SelfWatch │ │ MetaWatch │              │
│   └───────────┘ └───────────┘ └───────────┘ └───────────┘              │
│                    Consensus Engine (共识引擎)                          │
└─────────────────────────────────────────────────────────────────────────┘
                                 │ 驱动
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Response Layer                                 │
│                        (响应层 - 执行能力)                               │
│              31 AI Agents (工程 / 领域 / 思维)                          │
│              TypeScript (ccode) + Rust (zero-*)                        │
└─────────────────────────────────────────────────────────────────────────┘
```

### 架构特点

| 层级 | 职责 | 实现 |
|------|------|------|
| **Gear Control** | 用户控制自动化程度 | P/N/D/S/M + 三旋钮 |
| **Observer Network** | 持续观察，形成共识 | 四大观察者 + 共识引擎 |
| **Response Layer** | 响应观察结果 | 31 个专业 Agent |
```

**Step 2: Commit**

```bash
git add docs/architecture/ARCHITECTURE.md
git commit -m "docs: rewrite ARCHITECTURE.md opening to observation-centric"
```

---

## Phase 2: Terminology Updates (Batch Processing)

### Task 6: Update Terminology in DESIGN_PHILOSOPHY.md

**Files:**
- Modify: `docs/architecture/DESIGN_PHILOSOPHY.md`

**Step 1: Search and replace terminology**

| Find | Replace |
|------|---------|
| `个人工作台` | `AI 代理观察系统` |
| `Agent 系统` | `观察者网络 + 响应代理` |

**Step 2: Add observation-philosophy mapping section**

After the existing 祝融说 section, add:

```markdown
### 观察者架构与祝融说的映射

| 哲学概念 | 系统实现 | 代码位置 |
|----------|----------|----------|
| 可能性基底 | Observer Network 事件流 | `packages/ccode/src/observer/` |
| 观察即收敛 | Consensus Engine | `packages/ccode/src/observer/consensus/` |
| 可用余量 | Gear System 控制自由度 | `packages/ccode/src/observer/controller/` |
| 可持续决策 | CLOSE 评估框架 | `packages/ccode/src/observer/responders/` |
```

**Step 3: Commit**

```bash
git add docs/architecture/DESIGN_PHILOSOPHY.md
git commit -m "docs: add observation-philosophy mapping to DESIGN_PHILOSOPHY.md"
```

---

### Task 7: Update Terminology in CORE_CONCEPTS.md

**Files:**
- Modify: `docs/architecture/CORE_CONCEPTS.md`

**Step 1: Update the opening section**

Change the document title/intro to emphasize observation:

```markdown
# CodeCoder 核心概念

> 生成时间: 2026-03-09
> 架构层次: Gear Control → Observer Network → Response Layer

本文档梳理 CodeCoder 系统的核心概念，按观察者架构层次组织：

**控制层:** GEAR (档位控制)
**观察层:** OBSERVER (观察者网络), CONSENSUS (共识引擎)
**响应层:** AGENT, PROMPT, SKILL, TOOL, MEMORY
**接入层:** CHANNEL, WORKFLOW, HAND
```

**Step 2: Search and replace old terminology**

| Find | Replace |
|------|---------|
| `个人工作台` | `AI 代理观察系统` |

**Step 3: Commit**

```bash
git add docs/architecture/CORE_CONCEPTS.md
git commit -m "docs: update CORE_CONCEPTS.md to observation-centric structure"
```

---

### Task 8: Update Terminology in CCODE_VS_ZERO.md

**Files:**
- Modify: `docs/architecture/CCODE_VS_ZERO.md`

**Step 1: Update the relationship description**

Change the opening analogy from "智能中枢" to observation-centric:

```markdown
## 关系总览

核心关系:
- **ccode** 是"观察者大脑" — 运行 Observer Network，理解观察结果，驱动响应
- **zero-*** 是"感知器官和安全边界" — 接收外部信号，执行安全敏感操作
```

**Step 2: Commit**

```bash
git add docs/architecture/CCODE_VS_ZERO.md
git commit -m "docs: update CCODE_VS_ZERO.md with observation-centric terminology"
```

---

### Task 9: Batch Update Remaining Architecture Docs

**Files:**
- Modify: `docs/architecture/AGENT_FRAMEWORKS.md`
- Modify: `docs/architecture/SHARED_TYPES.md`
- Modify: `docs/architecture/ZERO_TRADING_WORKFLOW.md`
- Modify: `docs/architecture/README.md`

**Step 1: In each file, search and replace**

| Find | Replace |
|------|---------|
| `个人工作台` | `AI 代理观察系统` |
| `执行中心` | `观察中心` |

**Step 2: Commit all together**

```bash
git add docs/architecture/
git commit -m "docs: batch update architecture docs with observation terminology"
```

---

## Phase 3: Other Documentation Updates

### Task 10: Update docs/developer-guide.md

**Files:**
- Modify: `docs/developer-guide.md`

**Step 1: Update the project introduction section**

Find and replace terminology, ensure the guide mentions:
- CodeCoder is an observation-centric AI agent system
- Observer Network is the core
- Gear System controls automation

**Step 2: Commit**

```bash
git add docs/developer-guide.md
git commit -m "docs: update developer-guide.md with observation positioning"
```

---

### Task 11: Update docs/FEATURES.md

**Files:**
- Modify: `docs/FEATURES.md`

**Step 1: Reorganize features list**

Move observation-related features to the top:
1. Observer Network
2. Gear System
3. Response Agents
4. Infrastructure features

**Step 2: Commit**

```bash
git add docs/FEATURES.md
git commit -m "docs: reorganize FEATURES.md with observation features first"
```

---

### Task 12: Update docs/RUNBOOK.md and docs/DEBT.md

**Files:**
- Modify: `docs/RUNBOOK.md`
- Modify: `docs/DEBT.md`

**Step 1: Search and replace terminology in both files**

| Find | Replace |
|------|---------|
| `个人工作台` | `AI 代理观察系统` |

**Step 2: Commit**

```bash
git add docs/RUNBOOK.md docs/DEBT.md
git commit -m "docs: update RUNBOOK.md and DEBT.md terminology"
```

---

## Phase 4: Verification

### Task 13: Verify All Terminology Updates

**Step 1: Search for old terminology**

```bash
grep -r "个人工作台" docs/ README.md CLAUDE.md
```

Expected: No matches

**Step 2: Search for new terminology**

```bash
grep -r "观察系统\|观察中心\|Observer Network" docs/ README.md CLAUDE.md | wc -l
```

Expected: Multiple matches

**Step 3: Verify architecture diagrams show correct layer order**

Check that all architecture diagrams show:
- Gear Control Layer (top)
- Observer Network (middle)
- Response Layer (bottom)

---

### Task 14: Final Review and Commit

**Step 1: Review all changes**

```bash
git diff HEAD~12 --stat
```

**Step 2: Create summary commit if needed**

```bash
git add -A
git commit -m "docs: complete observation-centric documentation refactoring

- Rewrite README.md, CLAUDE.md, ARCHITECTURE.md with observation-first structure
- Update all architecture docs with new terminology
- Standardize on Gear → Observer → Response layer hierarchy
- Add philosophy-to-implementation mapping

Resolves: observation-centric documentation update"
```

---

## Summary

| Phase | Tasks | Documents |
|-------|-------|-----------|
| 1. Core Rewrites | 1-5 | README.md, CLAUDE.md, ARCHITECTURE.md |
| 2. Terminology | 6-9 | DESIGN_PHILOSOPHY.md, CORE_CONCEPTS.md, CCODE_VS_ZERO.md, others |
| 3. Other Docs | 10-12 | developer-guide.md, FEATURES.md, RUNBOOK.md, DEBT.md |
| 4. Verification | 13-14 | All |

**Total estimated tasks:** 14
**Total documents to modify:** ~15
