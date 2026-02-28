# CodeCoder 全自动自主解决问题流程图 - 生成报告

**生成时间**: 2026-02-28

## 概述

已为 CodeCoder 项目生成了 **9 个** 全自动自主解决问题流程图，涵盖从用户输入到任务完成的完整链路。

---

## 生成的流程图列表

| # | 文件名 | 描述 | 类型 |
|---|--------|------|------|
| 1 | `01-complete-system.svg` | 完整系统架构流程图 | Flowchart |
| 2 | `02-autonomous-execution.svg` | 自主执行模式核心流程 | Flowchart |
| 3 | `03-close-decision.svg` | CLOSE 决策框架评估流程 | Flowchart |
| 4 | `04-agent-delegation.svg` | Agent 委派与协作流程 | Flowchart |
| 5 | `05-workflow-engine.svg` | 工作流触发与执行流程 | Flowchart |
| 6 | `06-tool-calling-loop.svg` | 工具调用循环机制 | Flowchart |
| 7 | `07-memory-system.svg` | 记忆系统读写流程 | Flowchart |
| 8 | `08-determinism-split.svg` | 确定性 vs 不确定性任务分流 | Flowchart |
| 9 | `09-e2e-sequence.svg` | 端到端执行时序图 | Sequence |

---

## 文件位置

```
docs/diagrams/autonomous-flow/
├── *.mmd          # Mermaid 源文件
├── *.svg          # 渲染后的 SVG 图表
└── README.md      # 本文档
```

---

## 核心流程概览

### 1. 完整系统架构 (01)

展示了 CodeCoder 的五层架构：
- **用户接入层**: CLI、TUI、Web、IM 渠道
- **安全边界层**: Zero Gateway 认证/授权/配额/审计
- **核心引擎层**: 任务编排器、Agent 执行引擎、AI Provider、记忆系统
- **Rust 微服务层**: Daemon、Channels、Workflow、Trading、Browser
- **基础设施层**: Redis、SQLite、Docker

### 2. 自主执行核心流程 (02) ★

这是全自动自主解决问题的核心执行路径：

```
用户输入 → 意图识别 → CLOSE决策(可选) → 生成计划
    ↓
TDD循环 (RED → GREEN → IMPROVE) → 覆盖率检查
    ↓
代码审查 → 安全审查 → 验收测试 → 固化技能 → 完成
```

关键控制点：
- CLOSE 评估 Convergence≥7 或 Surplus≥7 可直接执行
- 测试覆盖率必须 ≥80%
- CRITICAL/HIGH 级别问题必须修复
- 安全审查必须通过

### 3. CLOSE 决策框架 (03)

CLOSE 五维评估流程，每个维度目标 ≥7，总分 ≥35 才批准执行：

| 维度 | 评估内容 | 改进措施 |
|------|----------|----------|
| C - Convergence | 问题收敛度 | 重新定义问题 |
| L - Leverage | 杠杆效应 | 寻找高杠杆点 |
| O - Optionality | 选择权保留 | 增加选项 |
| S - Surplus | 余量消耗 | 保留资源 |
| E - Evolution | 演化空间 | 模块化设计 |

### 4. Agent 委派协作 (04)

Autonomous Agent 可委派给 9 类专业 Agent：
- 代码审查 → code-reviewer
- 安全分析 → security-reviewer
- 新功能/修复 → tdd-guide
- 架构设计 → architect
- 代码库探索 → explore
- 长文写作 → writer
- 决策分析 → decision
- 多任务并行 → general
- 逆向工程 → code-reverse

### 5. 工作流引擎 (05)

支持 4 种触发方式：
- **Webhook**: HTTP 事件触发
- **Cron**: 定时任务触发
- **Git**: Push/PR 事件触发
- **Manual**: 用户手动触发

### 6. 工具调用循环 (06)

Agent 执行器的核心机制：
```
构建提示 → 调用LLM → 解析工具 → 执行工具 → 收集结果
    ↑                                              ↓
    ┄┄┄┄┄┄┄┄ 追加到历史 < 10次 且 无循环检测 ┄┄┄┄┄┄┄┄┘
```

安全机制：
- 最大迭代次数限制（10次）
- 循环检测机制
- 超时处理

### 7. 记忆系统 (07)

双层透明记忆架构：
- **流层** (Daily): `./memory/daily/YYYY-MM-DD.md`
- **沉积层** (Long-term): `./memory/MEMORY.md`

特点：Git 友好、人类可读、无需嵌入检索

### 8. 确定性/不确定性分流 (08)

**Rust 处理** (确定性强):
- 协议解析、安全验证、速率限制、消息路由、定时调度

**TypeScript 处理** (不确定性高):
- 意图理解、上下文推理、代码生成、决策建议

### 9. 端到端时序图 (09)

完整的用户请求到结果返回的时序流程，展示了 11 个组件之间的交互。

---

## 关键设计原则

### 1. 双层架构
```
┌─────────────────────────────────────────┐
│   TypeScript (ccode) - 智能层          │
│   • Agent编排 • LLM调用 • 记忆管理      │
├─────────────────────────────────────────┤
│   Rust (zero-*) - 边界层                │
│   • 安全认证 • 消息路由 • 工作流调度    │
└─────────────────────────────────────────┘
```

### 2. 自主执行七步循环
```
觉醒 → 扩张 → 创造 → 固化 → 验证 → 演化 → 觉醒
  ↑                                                    ↓
  └──────────────────────────────────────────────────┘
```

### 3. 可持续性 > 最优性
- 保持"再来一次"的能力 > 追求"最优解"
- 回滚机制、Plan Mode、Undo 能力
- 人在回路 (HITL) 关键决策确认

---

## 使用方式

### 在文档中使用 SVG

```markdown
![自主执行流程](../diagrams/autonomous-flow/02-autonomous-execution.svg)
```

### 修改图表

1. 编辑对应的 `.mmd` 文件
2. 重新渲染：
```bash
cd ~/.claude/skills/pretty-mermaid
node scripts/render.mjs \
  --input /path/to/diagram.mmd \
  --output /path/to/diagram.svg \
  --theme tokyo-night
```

### 批量重新渲染

```bash
cd ~/.claude/skills/pretty-mermaid
node scripts/batch.mjs \
  --input-dir /Users/iannil/Code/zproducts/code-coder/docs/diagrams/autonomous-flow \
  --output-dir /Users/iannil/Code/zproducts/code-coder/docs/diagrams/autonomous-flow \
  --format svg \
  --theme tokyo-night \
  --workers 4
```

---

`★ Insight ─────────────────────────────────────`
**全自动自主执行的核心设计**

1. **CLOSE 决策框架是"刹车"**: 通过五维评估确保每个决策都是可持续的，避免 AI 陷入局部最优或过度消耗资源。

2. **TDD + 代码审查 + 安全审查三重保障**: 即使是 AI 生成的代码，也必须经过严格的测试和审查流程，保证质量底线。

3. **技能固化是进化机制**: 每次成功解决问题的经验都会被写入 SKILL.md，形成长期记忆，实现系统级别的"演化"。
`─────────────────────────────────────────────────`
