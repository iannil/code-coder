# OpenFang 参考分析

> 生成时间: 2026-03-02
> 版本: 1.0.0

## 概述

本文档分析 CodeCoder 与 OpenFang 的设计差异，说明哪些特性已实现、哪些有意不借鉴，以及背后的设计哲学。

**核心结论**: CodeCoder 已经实现了 OpenFang 的核心特性（沙箱、Hands 自主代理、记忆系统），并在某些方面更为完善（如 6 级自治级别、CLOSE 框架集成）。

---

## 已实现特性

### 1. 沙箱系统

#### CodeCoder 实现

CodeCoder 提供三种沙箱后端：

| 后端 | 优势 | 适用场景 |
|------|------|----------|
| **Process** | 快速、易调试 | 开发/测试环境 |
| **Docker** | 完整容器隔离 | 生产环境 |
| **WASM** | 启动速度极快 (50x Docker) | JavaScript 代码 |

**资源限制**:
```typescript
// packages/ccode/src/autonomous/execution/sandbox.ts
const DEFAULT_LIMITS: Required<ResourceLimits> = {
  maxMemoryMb: 256,     // 内存限制
  maxTimeMs: 30000,     // 超时 30 秒
  allowNetwork: false,  // 禁止网络访问
  allowFileWrite: false, // 禁止文件写入
}
```

**Docker 容器隔离**:
- 只读文件系统
- 网络隔离
- 能力丢弃 (Capability Dropping)
- 资源配额 (CPU/内存)

**关键文件**:
- `packages/ccode/src/autonomous/execution/sandbox.ts` - 主沙箱执行器
- `packages/ccode/src/autonomous/execution/docker-sandbox.ts` - Docker 后端
- `packages/ccode/src/autonomous/execution/wasm-sandbox.ts` - WASM 后端 (QuickJS)

#### 与 OpenFang 对比

| 维度 | OpenFang | CodeCoder |
|------|----------|-----------|
| **沙箱类型** | WASM 双计量器 (Fuel + Epoch) | Process + Docker + WASM |
| **计量机制** | Fuel (指令) + Epoch (时间) | 简单超时 (maxTimeMs) |
| **性能** | ~180ms 冷启动 | ~500ms (Bun) / 50x 更快 (WASM) |
| **语言支持** | WASM 可编译语言 | Python, Node.js, Shell |

**设计选择**: CodeCoder 不采用 Fuel/Epoch 双计量，因为：
1. 个人工作台不需要精细的指令级计费
2. 简单超时机制已满足安全需求
3. 更容易理解和配置

---

### 2. Hands 自主代理系统

#### CodeCoder 实现

CodeCoder 的 Hands 系统在某些方面超越 OpenFang：

**触发器类型** (4 种):

| 触发器 | 类型 | 示例 |
|--------|------|------|
| **Cron** | 定时调度 | `0 9 * * *` 每日 9:00 |
| **Webhook** | HTTP 触发 | `/api/v1/hands/{id}/trigger` |
| **Git** | Git 事件 | push, pull_request, issue |
| **FileWatch** | 文件变化 | 监控文件系统变化 |

**自治级别** (6 级 - 比 OpenFang 更精细):

```typescript
// packages/ccode/src/autonomous/hands/bridge.ts
type AutonomyLevel =
  | "lunatic"  // 完全自主，无需审批
  | "insane"   // 高度自主，仅关键操作审批
  | "crazy"    // 自主运行，大部分操作自动批准
  | "wild"     // 中等自主，部分操作需审批
  | "bold"     // 保守自主，多数操作需审批
  | "timid"    // 最谨慎，几乎所有操作需审批
```

**风险控制** (5 级):

```typescript
// services/zero-workflow/src/hands/auto_approve.rs
type RiskLevel = "safe" | "low" | "medium" | "high" | "critical"

type ApprovalDecision =
  | "auto_approve"  // 自动批准执行
  | "queue"         // 进入人工审批队列
  | "deny"          // 拒绝执行
```

**Pipeline 模式** (多 Agent 协作):

```typescript
type PipelineMode =
  | "sequential"    // 顺序执行：前一个 Agent 的输出作为下一个的输入
  | "parallel"      // 并行执行：所有 Agent 同时执行并合并输出
  | "conditional"   // 条件执行：根据 CLOSE 框架决策选择下一个 Agent
```

**CLOSE 框架集成**:
- 五维决策评估 (Convergence, Leverage, Optionality, Surplus, Evolution)
- 每次执行自动生成 CLOSE 评分
- 质量评分 (quality_score) 和 疯狂度评分 (craziness_score)

**关键文件**:
- `packages/ccode/src/autonomous/hands/bridge.ts` - TypeScript 桥接
- `services/zero-workflow/src/hands/executor.rs` - Rust 执行器
- `services/zero-workflow/src/hands/auto_approve.rs` - 自动审批逻辑
- `services/zero-workflow/src/hands/scheduler.rs` - Cron 调度器

#### 与 OpenFang 对比

| 维度 | OpenFang | CodeCoder |
|------|----------|-----------|
| **内置 Hands** | 7 个 (Clip, Lead, Collector...) | Agent 矩阵组合 |
| **自治级别** | 未明确分级 | 6 级精细分级 |
| **风险控制** | 审批队列 | 5 级风险 + 超时自动批准 |
| **决策框架** | 无 | CLOSE 五维评估 |
| **多 Agent Pipeline** | 未明确 | Sequential/Parallel/Conditional |

---

### 3. 记忆系统

#### CodeCoder 选择

CodeCoder 采用 **双层 Markdown 架构**，而非 OpenFang 的 SQLite + 向量嵌入：

**第一层: 每日笔记 (流/Stream)**
```
memory/daily/YYYY-MM-DD.md
```
- 仅追加、不可修改
- 按时间线记录
- 类比：河流 (flow)

**第二层: 长期记忆 (沉积/Sediment)**
```
memory/MEMORY.md
```
- 可编辑、结构化
- 代表当前真实状态
- 类比：沉积岩

**关键文件**:
- `packages/ccode/src/memory-markdown/` - Markdown 记忆实现
- `packages/ccode/src/memory-markdown/daily.ts` - 每日笔记
- `packages/ccode/src/memory-markdown/long-term.ts` - 长期记忆
- `packages/ccode/src/memory-markdown/consolidate.ts` - 知识沉积

#### 为什么选择 Markdown 而非 SQLite？

| 维度 | OpenFang (SQLite + 向量) | CodeCoder (Markdown) |
|------|-------------------------|---------------------|
| **人类可读** | 需要工具查询 | 直接编辑 |
| **Git 友好** | 二进制数据库，不可 diff | 纯文本，可 diff |
| **调试方式** | SQL 查询 | 直接编辑 .md 文件 |
| **透明性** | 需要专业工具 | 任何文本编辑器 |
| **语义搜索** | 向量嵌入支持 | 有意不采用 |

**设计哲学**: "人机透明" (Human-Machine Transparency)
- 记忆文件对人类可读
- 对 Git 友好，变化可追踪
- 无需复杂检索工具，降低系统复杂度

---

## 不借鉴的特性及原因

### 1. 纯 Rust 架构 → TS 混合

**OpenFang**: 137K LOC Rust，单一二进制
**CodeCoder**: TypeScript 60% + Rust 40%

**不借鉴原因**:
- TypeScript 提供更快的迭代速度
- 高层抽象用 TS，高性能服务用 Rust
- 符合 "高不确定性任务用 LLM，高确定性任务用 Rust" 的划分原则

### 2. SQLite + 向量嵌入 → Markdown

**OpenFang**: SQLite 数据库 + 向量存储
**CodeCoder**: 纯 Markdown 文件

**不借鉴原因**: (见上文"记忆系统"部分)

### 3. 40+ 通道 → 专注核心能力

**OpenFang**: 40+ IM 渠道适配器
**CodeCoder**: 12+ 核心渠道

**不借鉴原因**:
- 个人工作台不需要全部渠道
- 专注常用渠道 (Telegram, Discord, Slack, Email)
- 保持代码可维护性

### 4. Merkle 审计链 → 简化审计

**OpenFang**: 16 层安全 + Merkle 哈希链审计
**CodeCoder**: 日志 + SQLite + RBAC

**不借鉴原因**:
- 个人工作台不需要企业级审计
- Git 已提供变更追踪
- Merkle 链增加复杂度，收益有限

### 5. OFP 网络协议 → HTTP + MCP

**OpenFang**: 自定义 P2P 网络协议 (OFP)
**CodeCoder**: HTTP API + MCP (Model Context Protocol)

**不借鉴原因**:
- 使用标准协议降低学习成本
- MCP 是 AI 领域的开放标准
- HTTP 更易于调试和集成

---

## 关键文件索引

### 沙箱系统
```
packages/ccode/src/autonomous/execution/
├── sandbox.ts           # 主沙箱执行器
├── docker-sandbox.ts    # Docker 后端
└── wasm-sandbox.ts      # WASM 后端
```

### Hands 自主代理
```
packages/ccode/src/autonomous/hands/
└── bridge.ts            # TypeScript 桥接层

services/zero-workflow/src/hands/
├── executor.rs          # 执行引擎
├── auto_approve.rs      # 自动审批逻辑
├── scheduler.rs         # Cron 调度器
└── manifest.rs          # HAND.md 解析器
```

### 记忆系统
```
packages/ccode/src/memory-markdown/
├── daily.ts             # 每日笔记
├── long-term.ts         # 长期记忆
├── consolidate.ts       # 知识沉积
└── index.ts             # 入口
```

### 自治循环
```
packages/ccode/src/autonomous/execution/
└── evolution-loop.ts    # 自主问题求解循环
```

---

## 附录: OpenFang Hands 列表

OpenFang 内置的 7 个 Hands，供参考：

| Hand | 功能 | CodeCoder 对应方案 |
|------|------|-------------------|
| **Clip** | YouTube 视频剪辑、字幕生成 | `browser` + `writer` Agent 组合 |
| **Lead** | 每日运行，发现/评分/去重潜在客户 | `picker` + `macro` Agent 组合 |
| **Collector** | OSINT 级情报收集，变化检测 | `explore` + 搜索工具 |
| **Predictor** | 超级预测引擎，Brier 评分跟踪 | `trader` + `macro` Agent 组合 |
| **Researcher** | 深度研究，CRAAP 标准评估 | `writer` + `explore` Agent 组合 |
| **Twitter** | 自主 Twitter 账号管理，发布审批队列 | `channels` + Webhook 触发 |
| **Browser** | 网页自动化，购买强制审批 | `zero-browser` 服务 |

**结论**: CodeCoder 的 Agent 矩阵 + Hands 系统可以实现 OpenFang 所有内置 Hand 的功能，且更灵活。

---

## 变更历史

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-03-02 | 1.0.0 | 初始版本 |
