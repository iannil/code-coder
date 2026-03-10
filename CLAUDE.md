# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此代码库中工作时提供指导。

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

## 观察者网络 (Observer Network)

观察者网络将 CodeCoder 从执行中心系统转变为观察中心系统，体现"祝融说"哲学：

- **可能性基底** (Possibility Substrate): 原始观察事件流
- **观察即收敛** (Observation as Convergence): 共识形成机制
- **可用余量** (Available Margin): 模式切换自由度
- **评估权** (Evaluation Authority): 人类干预点

### 架构概览

```
┌─────────────────────────────────────────────────────────────────────┐
│                    观察者层 (Observer Layer)                        │
│   CodeWatch │ WorldWatch │ SelfWatch │ MetaWatch                   │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    事件流 (Event Stream)                            │
│   缓冲 │ 路由 │ 聚合                                                │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    共识层 (Consensus Layer)                         │
│   注意力 │ 模式检测 │ 异常检测 │ 世界模型                           │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    模式控制器 (Mode Controller)                     │
│   AUTO │ MANUAL │ HYBRID │ CLOSE 评估                              │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    响应层 (Response Layer)                          │
│   Notifier │ Analyzer │ Executor │ Historian                       │
└─────────────────────────────────────────────────────────────────────┘
```

### 四大观察者节点

| 观察者 | 职责 | 关联 Agent |
|--------|------|-----------|
| CodeWatch | 代码库扫描、Git 变更、构建状态 | explore |
| WorldWatch | 市场数据、新闻舆情、API 变化 | macro, trader |
| SelfWatch | Agent 行为、决策日志、错误模式 | code-reviewer, security-reviewer, decision |
| MetaWatch | 观察质量、系统健康、观察盲点 | observer |

### 操作模式

- **AUTO**: 完全自主执行，适用于高置信度、低风险场景
- **MANUAL**: 需要人类确认，适用于关键决策
- **HYBRID**: 自动执行 + 事后确认，平衡效率与控制

模式切换基于 CLOSE 五维评估 (Convergence, Leverage, Optionality, Surplus, Evolution)。

### 使用示例

```typescript
import { ObserverNetwork } from "@/observer"

// 启动观察者网络
const network = await ObserverNetwork.start({
  mode: "HYBRID",
  riskTolerance: "balanced",
  watchers: {
    code: { enabled: true },
    world: { enabled: true },
    self: { enabled: true },
    meta: { enabled: true },
  },
})

// 订阅观察事件
network.onObservation((obs) => {
  console.log(`[${obs.watcherType}] ${obs.type}`)
})

// 获取当前世界模型
const model = await network.getWorldModel()

// 切换模式
await network.switchMode("MANUAL", "用户请求")

// 停止网络
await network.stop()
```

### 关键文件

- `packages/ccode/src/observer/` - 观察者网络模块
- `packages/ccode/src/observer/watchers/` - 四大观察者实现
- `packages/ccode/src/observer/consensus/` - 共识引擎
- `packages/ccode/src/observer/controller/` - 模式控制器
- `packages/ccode/src/observer/responders/` - 响应组件

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

## 响应代理系统 (Agent System)

Agent 系统采用 3 模式设计，简化用户选择：

**@build 模式 (默认) - 软件开发**
- 主 Agent: `build`
- 备选: `plan`, `autonomous`
- 能力: `code-reviewer`, `security-reviewer`, `tdd-guide`, `architect`, `explore`, `general`, `code-reverse`, `jar-code-reverse`, `verifier`, `prd-generator`, `feasibility-assess`

**@writer 模式 - 内容创作**
- 主 Agent: `writer`
- 能力: `expander`, `expander-fiction`, `expander-nonfiction`, `proofreader`, `verifier`

**@decision 模式 - 决策与哲学 (祝融说)**
- 主 Agent: `decision`
- 备选: `observer`
- 能力: `macro`, `trader`, `value-analyst`, `picker`, `miniproduct`, `ai-engineer`, `synton-assistant`

**系统隐藏 (不对用户显示):**
`compaction`, `title`, `summary`

**使用方式:**
```bash
bun dev              # 默认 @build 模式
bun dev -m writer    # 写作模式
bun dev -m decision  # 决策模式

# 访问特定能力
bun dev @build:security-review
bun dev @decision:macro
```

### 29 个 Agent 概览

**主模式 (4):**
build、plan、writer、autonomous

**逆向工程 (2):**
code-reverse、jar-code-reverse

**工程质量 (6):**
general、explore、code-reviewer、security-reviewer、tdd-guide、architect

**内容创作 (3):**
expander (支持 fiction/nonfiction 自动检测)、proofreader、verifier

**祝融说系列 ZRS (8):**
observer、decision、macro、trader、picker、miniproduct、ai-engineer、value-analyst

**产品与可行性 (2):**
prd-generator、feasibility-assess

**其他 (1):**
synton-assistant

**系统隐藏 (3):**
compaction、title、summary

## 项目指南

- 目标：以强类型、可测试、分层解耦为核心，保证项目健壮性与可扩展性；以清晰可读、模式统一为核心，使大模型易于理解与改写。
- 语言约定：交流与文档使用中文；生成的代码使用英文；文档放在 `docs` 且使用 Markdown。
- 发布约定：
  - 发布固定在 `/release` 文件夹，如 rust 服务固定发布在 `/release/rust` 文件夹。
  - 发布的成果物必须且始终以生产环境为标准，要包含所有发布生产所应该包含的文件或数据（包含全量发布与增量发布，首次发布与非首次发布）。
- 环境约定：
  - 尽量使用docker部署环境
  - 尽量为项目配置独立的网络，避免与其他项目网络冲突
- 文档约定：
  - 每次修改都必须延续上一次的进展，每次修改的进展都必须保存在对应的 `docs` 文件夹下的文档中。
  - 执行修改过程中，进展随时保存文档，带上实际修改的时间，便于追溯修改历史。
  - 未完成的修改，文档保存在 `/docs/progress` 文件夹下。
  - 已完成的修改，文档保存在 `/docs/reports/completed` 文件夹下。
  - 对修改进行验收，文档保存在 `/docs/reports` 文件夹下。
  - 对重复的、冗余的、不能体现实际情况的文档或文档内容，要保持更新和调整。
  - 文档模板和命名规范可以参考 `/docs/standards` 和 `docs/templates` 文件夹下的内容。
- 文件夹约定：
  - `memory`、`docs`、`example`文件夹只能存储于项目根目录`/`之下，不允许存储在项目任何其他位置。

### 自主任务规范

当 Agent 执行自主任务（如来自 Telegram 消息、定时任务或后台任务）时，必须遵循以下规范：

- **文件位置**：自主任务产生的文件只能存储在 workspace 目录 (`~/.codecoder/workspace`)
- **禁止区域**：严禁在 `packages/`、`services/`、`src/`、`scripts/` 目录下创建文件
- **能力优先级**：
  1. 首选：调用现有 Agent（macro/trader/picker 等）和 API 组合
  2. 其次：调用现有工具和服务（scheduler、channels）
  3. 最后：仅在必要时创建新脚本（需用户明确确认）
- **定时任务**：必须使用 Scheduler API (`/api/v1/scheduler/tasks`) 或 `scheduler_create_task` 工具，禁止直接创建 crontab 或 shell 脚本
- **数据获取**：使用对应领域的 Agent（macro 获取财经、trader 获取行情），禁止硬编码假数据

### 面向大模型的可改写性（LLM Friendly）

- 一致的分层与目录：相同功能在各应用/包中遵循相同结构与命名，使检索与大范围重构更可控。
- 明确边界与单一职责：函数/类保持单一职责；公共模块暴露极少稳定接口；避免隐式全局状态。
- 显式类型与契约优先：导出 API 均有显式类型；运行时与编译时契约一致（zod schema 即类型源）。
- 声明式配置：将重要行为转为数据驱动（配置对象 + `as const`/`satisfies`），减少分支与条件散落。
- 可搜索性：统一命名（如 `parseXxx`、`assertNever`、`safeJsonParse`、`createXxxService`），降低 LLM 与人类的检索成本。
- 小步提交与计划：通过 `IMPLEMENTATION_PLAN.md` 和小步提交让模型理解上下文、意图与边界。
- 变更安全策略：批量程序性改动前先将原文件备份至 `/backup` 相对路径；若错误数异常上升，立即回滚备份。

### 可观测性开发（Observability Driven Development）

- 为了能够完整追踪代码的执行流，请你遵循 "全链路可观测性 (Full-Lifecycle Observability)" 模式编写代码；
- 结构化日志： 所有的日志输出必须是 JSON 格式，包含字段：timestamp, trace_id (全链路唯一ID), span_id (当前步骤ID), event_type (Function_Start/End, Branch, Error), payload (变量状态)；
- 装饰器/切面模式： 请定义一个 LifecycleTracker 装饰器或上下文管理器；
- 在函数进入时：记录输入参数 (Args/Kwargs)；
- 在函数退出时：记录返回值 (Return Value) 和耗时 (Duration)；
- 在函数异常时：记录完整的堆栈信息 (Stack Trace)；
- 关键节点埋点： 在复杂的 if/else 分支、for/while 循环内部、以及外部 API 调用前后，必须手动添加埋点（Point）；
- 执行摘要： 代码运行结束时，必须能够生成一份“执行轨迹报告 (Execution Trace Report)”；
- 请确保埋点代码与业务逻辑解耦（尽量使用装饰器），不要让日志代码淹没业务逻辑；

### 记忆系统

本项目采用基于Markdown文件的透明双层记忆架构。禁止使用复杂的嵌入检索。 所有记忆操作必须对人类可读且对Git友好。

#### 存储结构

记忆分为两个独立的层："流"（日常）层和"沉积"（长期）层。

- 第一层：每日笔记（流）
  - 路径： `/memory/daily/{YYYY-MM-DD}.md`
  - 类型： 仅追加日志。
  - 目的： 记录上下文的"流动"。今天所说的一切、做出的决定以及完成的任务。
  - 格式： 按时间顺序排列的Markdown条目。

- 第二层：长期记忆（沉积）
  - 路径： `/memory/MEMORY.md`
  - 类型： 经过整理、结构化的知识。
  - 目的： 记录上下文的"沉积"。用户偏好、关键上下文、重要决策以及"经验教训"（避免过去的错误）。
  - 格式： 分类的Markdown（例如 `## 用户偏好`、`## 项目上下文`、`## 关键决策`）。

#### 操作规则

##### 上下文加载（读取）

当初始化会话或生成响应时，通过组合以下内容来构建系统提示：

1. 长期上下文： 读取 `MEMORY.md` 的全部内容。
2. 近期上下文： 读取当前（以及可选的之前）一天的每日笔记内容。

##### 记忆持久化（写入）

- 即时操作（日常）：
  - 将每一次重要的交互、工具输出或决策追加到当天的每日笔记中。
  - 不要覆盖或删除每日笔记中的内容；将其视为不可变的日志。
- 整合操作（长期）：
  - 触发条件： 当检测到有意义的信息时（例如，用户陈述了偏好、发现了特定的错误修复模式、建立了项目规则）。
  - 操作： 更新 `MEMORY.md`。
  - 方法： 智能地将新信息合并到现有类别中。如果信息已过时，则移除或更新它。此文件代表*当前*的真实状态。

#### 维护与调试

- 透明度： 所有记忆文件都是标准的Markdown文件。如果代理因错误的上下文而行为异常，修复方法是手动编辑 `.md` 文件。
- 版本控制： 所有记忆文件都受Git跟踪。

## 开发命令

```bash
# 安装依赖（需要 Bun 1.3+）
bun install

# 在当前目录运行 CodeCoder TUI
bun dev

# 在指定目录运行
bun dev <path>

# 启动无头 API 服务器（默认端口 4400）
bun dev serve
bun dev serve --port 4400

# 运行所有包的类型检查
bun turbo typecheck

# 运行测试（必须在特定包内运行，不能从根目录运行）
cd packages/ccode && bun test

# 构建独立可执行文件
bun run --cwd packages/ccode build

# API 更改后重新生成 SDK
./script/generate.ts
```

### 端口配置

**核心服务 (4400-4409):**

- CodeCoder API Server: 4400 (Bun/TypeScript)
- Web Frontend (Vite): 4401 (React)
- Zero CLI Daemon: 4402 (Rust, 统一服务入口)
  - `/health` - 健康检查
  - `/gateway/*` - 认证、路由、配额
  - `/channels/*` - IM 渠道适配
  - `/workflow/*` - 调度、Webhook、自动化
- Faster Whisper Server: 4403 (Docker)

**基础设施服务 (4410-4419):**

- Redis Server: 4410 (Docker, 会话存储, 可选)

**协议服务 (4420-4429):**

- MCP Server (HTTP): 4420 (Model Context Protocol)

> **注**: 所有服务通过 zero-cli daemon (:4402) 的路径前缀统一访问，无需记忆多个端口。

#### 服务架构 (4 Crates)

```
┌───────────────────────────────────────────────────────────────────────────────────────┐
│                              zero-cli daemon :4402                                     │
│                          (统一入口 + 进程编排)                                          │
│       职责: HTTP 服务、健康检查、自动重启、日志聚合                                       │
│       路由: /gateway/*, /channels/*, /workflow/*                                       │
│                                   │                                                    │
│          ┌────────────────────────┼────────────────────────┐                          │
│          │                        │                        │                          │
│          ▼                        ▼                        ▼                          │
│    ┌──────────┐            ┌──────────┐            ┌──────────┐                       │
│    │zero-core │            │zero-hub  │            │zero-     │                       │
│    │(工具库)  │            │(服务中枢)│            │trading   │                       │
│    │          │            │          │            │(交易)    │                       │
│    │tools/    │            │gateway/  │            │          │                       │
│    │session/  │            │channels/ │            │          │                       │
│    │security/ │            │workflow/ │            │          │                       │
│    │common/   │            │          │            │          │                       │
│    └──────────┘            └──────────┘            └──────────┘                       │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

> **注**: zero-common 已合并到 zero-core/src/common/，作为共享类型、配置和工具库。

**zero-hub 内部模块:**

- `gateway/` - 认证 (Pairing + JWT)、RBAC、配额、审计
- `channels/` - IM 渠道 (Telegram/Discord/Slack/飞书/Email/iMessage)
- `workflow/` - 调度 (Cron)、Webhook、Git 集成、Hands 自主执行

### 运维脚本

```bash
./ops.sh start          # 开发模式: daemon 自动管理微服务
./ops.sh start all      # 启动所有服务
./ops.sh stop           # 停止所有服务
./ops.sh status         # 查看服务状态
./ops.sh build rust     # 构建 Rust 服务
./ops.sh logs zero-daemon  # 查看日志
./ops.sh health         # 健康检查
```

### 统一配置

配置文件位于 `~/.codecoder/`，支持模块化文件结构：

```
~/.codecoder/
├── config.json           # 核心配置 (~80 行)
├── secrets.json          # 凭证文件 (gitignored, 600权限)
├── trading.json          # 交易模块配置
├── channels.json         # IM渠道配置
└── providers.json        # LLM提供商配置
```

**核心原则**:
- 配置使用 JSON 格式，禁止使用 `.toml`
- `secrets.json` 单独存储敏感信息，权限设置为 600
- 模块化文件会自动合并到主配置，优先级高于 `config.json` 中的同名字段
- 环境变量 (`ANTHROPIC_API_KEY`, `ZERO_*`) 具有最高优先级

**Schema 验证**:
- 所有配置文件均有对应的 JSON Schema: `schemas/*.schema.json`
- 运行 `bun run script/generate-config.ts` 可从 Schema 生成 TypeScript 类型

**迁移旧配置**:
```bash
# 预览变更
bun run script/migrate-config.ts --dry-run

# 执行迁移
bun run script/migrate-config.ts
```

## 架构

> **核心原则**: 高确定性任务用 zero-* (Rust) 保证效率；高不确定性任务用 ccode (LLM) 保证正确反应。
>
> 详细架构文档见 `docs/architecture/`:
>
> - `ARCHITECTURE.md` - 整体架构概览
> - `CCODE_VS_ZERO.md` - ccode 与 zero-* 的关系和职责划分
> - `DESIGN_PHILOSOPHY.md` - 设计哲学和原则

### 确定性 vs 不确定性划分

| 任务类型 | 最佳工具 | 原因 |
|---------|---------|------|
| 协议解析、签名验证、调度 | zero-* (Rust) | 规则明确，需要高性能和安全性 |
| 意图理解、代码生成、决策建议 | ccode (LLM) | 需要推理和领域知识 |

混合模式参考实现：`services/zero-trading/src/macro_agent/orchestrator.rs` (MacroOrchestrator)

通用 trait：`services/zero-core/src/common/hybrid.rs` (HybridDecisionMaker)

### Monorepo 结构

**TypeScript Packages (packages/):**

- `packages/ccode/` - 核心 CLI 工具和业务逻辑。入口点是 `src/index.ts`。包含主要的 agent 实现、LSP 集成和服务器。
- `packages/ccode/src/cli/cmd/tui/` - 终端 UI 代码，使用 SolidJS 和 [opentui](https://github.com/sst/opentui) 编写
- `packages/web/` - Web 前端 (React + Vite)
- `packages/util/` - 共享工具
- `script/` - 项目级构建和生成脚本

**Rust Services (services/) - 4 Crates:**

- `services/zero-cli/` - CLI + Daemon (统一入口)，依赖 zero-core, zero-hub
- `services/zero-core/` - 核心工具库 (grep/glob/edit, NAPI 绑定, MCP/LSP 协议) + 共享模块 (原 zero-common)
- `services/zero-hub/` - 统一服务中枢，包含:
  - `src/gateway/` - 认证、路由、配额、RBAC、审计
  - `src/channels/` - IM 渠道 (Telegram/Discord/Slack/飞书/Email)
  - `src/workflow/` - 调度、Webhook、Git、Hands 自主执行
- `services/zero-trading/` - PO3+SMT 交易系统

### 核心技术

- 运行时： Bun 1.3+ (TypeScript), Rust 1.75+ (Services)
- 构建： Turborepo (TS), Cargo Workspace (Rust)
- 前端： React (Web)、Solid.js + OpenTUI（终端）、TailwindCSS
- 后端： Hono (TS HTTP)、Axum (Rust HTTP)、Cloudflare Workers
- AI： 多个提供商 SDK（Anthropic、OpenAI、Google 等）、MCP 协议

### SDK 生成

JavaScript SDK 从 OpenAPI 规范自动生成。修改 API 后，运行 `./script/generate.ts` 重新生成。

## Agent 架构

### Agent 定义位置

- 核心 Agent 定义：`packages/ccode/src/agent/agent.ts`
- Prompt 文件目录：`packages/ccode/src/agent/prompt/`

### Agent 分类

| 分类 | Agent | 用途 |
| ------ | ------- | ------ |
| 主模式 | build, plan | 主要开发模式 |
| 逆向工程 | code-reverse, jar-code-reverse | 代码逆向分析 |
| 工程质量 | code-reviewer, security-reviewer, tdd-guide, architect | 代码质量保障 |
| 内容创作 | writer, proofreader | 长文写作与校对 |
| 祝融说系列 | observer, decision, macro, trader, picker, miniproduct, ai-engineer | 决策与领域咨询 |
| 工具辅助 | explore, general, synton-assistant | 探索与辅助 |

### 使用场景示例

代码审查

```
> "Review the recent changes for security issues"
```

系统自动使用 security-reviewer agent 进行分析。

决策咨询

```
> "@decision 用CLOSE框架分析这个职业选择"
```

使用 CLOSE 五维评估法分析决策。

领域分析

```
> "@macro 解读本月的 PMI 数据"
```

使用宏观经济分析框架解读数据。

## 代码风格指南

尽可能始终使用并行工具。

- 避免使用 `let` 语句 - 优先使用 `const` 和三元运算符
- 避免使用 `else` 语句 - 使用提前返回
- 避免不必要的解构 - 使用 `obj.a` 而不是 `const { a } = obj`
- 尽可能避免 `try`/`catch` - 优先使用 `.catch()`
- 避免使用 `any` 类型
- 优先使用单字变量名
- 除非可组合/可重用，否则将逻辑保持在一个函数中
- 尽可能使用 Bun API（例如 `Bun.file()`）

## 测试

- 测试文件位于各包内的 `test/` 目录中
- 使用 Bun 内置测试运行器
- 从特定包运行：`cd packages/ccode && bun test`
- 不要从仓库根目录运行测试

## 格式化

- Prettier： 120 字符宽度，无分号
- EditorConfig： 2 空格缩进，最大 80 字符行宽，LF 换行符

## 贡献

- PR 必须引用现有的 issue
- PR 标题遵循约定式提交规范（`feat:`、`fix:`、`docs:` 等）
- UI 更改需要提供截图/视频
- 所有 PR 应保持小而专注
- PR 描述中不要使用 AI 生成的长篇大论
