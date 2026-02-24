# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此代码库中工作时提供指导。

## 项目概述

CodeCoder 是一个个人工作台，融合工程能力与决策智慧。

使用 Turborepo 和 Bun 构建的 monorepo，支持多个 AI 提供商（Claude、OpenAI、Google、通过 MCP 支持本地模型）、CLI/TUI 界面，采用客户端/服务器架构支持远程操作。

### 核心定位

1. 工程层：代码审查、安全分析、TDD、架构设计、逆向工程
2. 领域层：宏观经济、交易分析、选品策略、极小产品、AI 工程
3. 思维层：祝融说哲学体系、CLOSE 决策框架、观察者理论

### 哲学框架

本项目内置基于"祝融说"的决策与认知系统，核心理念：

- 可能性基底：宇宙的终极实在是包含一切潜能的无限场域
- 观察即收敛：观察是创造性行为，导致可能性"坍缩"为确定性
- 可用余量：尚未被固化的潜能空间，是自由意志和创造力的来源
- 可持续决策 > 最优决策：保持"再来一次"的能力比追求"最优解"更重要

### 23 个 Agent 概览

主模式：build、plan、code-reverse、jar-code-reverse

工程类：general、explore、code-reviewer、security-reviewer、tdd-guide、architect

内容类：writer、proofreader

祝融说系列（ZRS）：observer、decision、macro、trader、picker、miniproduct、ai-engineer

其他：synton-assistant

系统隐藏：compaction、title、summary

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
  - 路径： `./memory/daily/{YYYY-MM-DD}.md`
  - 类型： 仅追加日志。
  - 目的： 记录上下文的"流动"。今天所说的一切、做出的决定以及完成的任务。
  - 格式： 按时间顺序排列的Markdown条目。

- 第二层：长期记忆（沉积）
  - 路径： `./memory/MEMORY.md`
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
- Zero CLI Daemon: 4402 (Rust, 进程编排器)
- Faster Whisper Server: 4403 (Docker)

**协议服务 (4420-4429):**

- MCP Server (HTTP): 4420 (Model Context Protocol)

**Rust 微服务 (4430-4439):**

- Zero Gateway: 4430 (统一网关: 认证/路由/配额/MCP/Webhook)
- Zero Channels: 4431 (IM 渠道: Telegram/Discord/Slack)
- Zero Workflow: 4432 (工作流: Webhook/Cron/Git)

#### 服务架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      zero-cli daemon 命令                        │
│                        (进程编排器)                              │
│  职责: spawn 子进程、健康检查、自动重启、日志聚合                  │
│              │                │                │                │
│              ▼                ▼                ▼                │
│       ┌──────────┐     ┌──────────┐     ┌──────────┐           │
│       │  zero-   │     │  zero-   │     │  zero-   │           │
│       │ gateway  │     │ channels │     │ workflow │           │
│       │  :4430   │     │  :4431   │     │  :4432   │           │
│       └──────────┘     └──────────┘     └──────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

**Gateway 功能:**
- 配对码认证 (Pairing) + JWT 双模式
- Webhook 端点 (/webhook)
- MCP JSON-RPC 端点 (/mcp)
- 用户管理 CRUD + RBAC
- 配额管理和审计日志
- Tunnel 支持 (Cloudflare/Tailscale/ngrok)

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

- 所有服务的配置文件统一为：`~/.codecoder/config.json`
- 禁止使用`.toml`格式

## 架构

### Monorepo 结构

**TypeScript Packages (packages/):**

- `packages/ccode/` - 核心 CLI 工具和业务逻辑。入口点是 `src/index.ts`。包含主要的 agent 实现、LSP 集成和服务器。
- `packages/ccode/src/cli/cmd/tui/` - 终端 UI 代码，使用 SolidJS 和 [opentui](https://github.com/sst/opentui) 编写
- `packages/web/` - Web 前端 (React + Vite)
- `packages/util/` - 共享工具
- `script/` - 项目级构建和生成脚本

**Rust Services (services/):**

- `services/zero-cli/` - Zero CLI 主程序，包含 daemon 命令（组合 gateway + channels + scheduler）
- `services/zero-gateway/` - 独立网关服务（认证、路由、配额、安全沙箱）
- `services/zero-channels/` - 独立频道服务（Telegram、Discord、Slack、Email）
- `services/zero-workflow/` - 独立工作流服务（Webhook、Cron、Git 集成）
- `services/zero-agent/` - Agent 执行逻辑（库）
- `services/zero-memory/` - 内存/持久化（库）
- `services/zero-tools/` - 工具定义（库）
- `services/zero-common/` - 共享配置和工具（库）

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
